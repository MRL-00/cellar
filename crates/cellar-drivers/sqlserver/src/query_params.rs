use std::{collections::HashMap, time::Instant};

use cellar_core::error::{CellarError, CellarResult};
use cellar_core::query::{NoticeCapture, Query, QueryResult};
use cellar_core::value::{CellValue, ColumnMeta};
use futures_util::TryStreamExt;
use tiberius::{Query as TdsQuery, QueryItem};

use super::{DEFAULT_MAX_ROWS, quote_ident, run_control};
use crate::connect::{
    SqlServerConnection, TdsClient, map_tiberius_runtime_err, session_invalidated,
};
use crate::decode::decode_cell;

pub(super) async fn execute_query(
    conn: &SqlServerConnection,
    query: &Query,
) -> CellarResult<QueryResult> {
    let max_rows = query.max_rows.unwrap_or(DEFAULT_MAX_ROWS) as usize;
    let offset = query.offset.unwrap_or(0) as usize;
    let started = Instant::now();
    let (sql, values) = prepare_query(query)?;
    let home_db = conn.config().database.clone();
    let target_db = query
        .database
        .as_deref()
        .filter(|database| !database.is_empty())
        .unwrap_or(home_db.as_str());

    conn.with_client(async |client| {
        let switched = target_db != home_db.as_str();
        if switched {
            run_control(client, &format!("USE {}", quote_ident(target_db))).await?;
        }
        let result = execute_sql(
            client,
            sql.as_str(),
            values.as_slice(),
            max_rows,
            offset,
            started,
        )
        .await;
        if switched {
            let restore = run_control(client, &format!("USE {}", quote_ident(&home_db))).await;
            if let Err(error) = restore {
                return Err(session_invalidated(format!(
                    "could not restore database [{home_db}] after parameterized query: {error}"
                )));
            }
        }
        result
    })
    .await
}

async fn execute_sql(
    client: &mut TdsClient,
    sql: &str,
    values: &[&CellValue],
    max_rows: usize,
    offset: usize,
    started: Instant,
) -> CellarResult<QueryResult> {
    let mut statement = TdsQuery::new(sql.to_owned());
    for value in values {
        bind_value(&mut statement, value)?;
    }
    let mut stream = statement
        .query(client)
        .await
        .map_err(|e| map_tiberius_runtime_err(e, "parameterized query execution"))?;
    let mut columns: Option<Vec<ColumnMeta>> = None;
    let mut rows = Vec::with_capacity(max_rows.min(10_000));
    let mut rows_seen = 0usize;
    let mut truncated = false;
    while let Some(item) = stream
        .try_next()
        .await
        .map_err(|e| map_tiberius_runtime_err(e, "parameterized query execution"))?
    {
        match item {
            QueryItem::Metadata(meta) if columns.is_none() => {
                columns = Some(
                    meta.columns()
                        .iter()
                        .map(|column| ColumnMeta {
                            name: column.name().to_string(),
                            data_type: format!("{:?}", column.column_type()).to_lowercase(),
                            nullable: true,
                        })
                        .collect(),
                );
            }
            QueryItem::Row(row) => {
                if rows_seen < offset {
                    rows_seen += 1;
                    continue;
                }
                if rows.len() >= max_rows {
                    truncated = true;
                    break;
                }
                rows.push(row.into_iter().map(decode_cell).collect());
                rows_seen += 1;
            }
            _ => {}
        }
    }
    Ok(QueryResult {
        columns: columns.unwrap_or_default(),
        rows,
        notices: Vec::new(),
        notice_capture: NoticeCapture::unsupported(
            "SQL Server informational messages are not exposed through the current tiberius query path.",
        ),
        rows_affected: None,
        duration_ms: started.elapsed().as_millis() as u64,
        truncated,
        total_rows: None,
    })
}

fn prepare_query(query: &Query) -> CellarResult<(String, Vec<&CellValue>)> {
    let prepared = cellar_sql::prepare_native(&query.sql, cellar_core::driver::Engine::Mssql)
        .map_err(|error| CellarError::query(error.to_string()))?;
    let by_name: HashMap<&str, &CellValue> = query
        .params
        .iter()
        .map(|param| (param.name.as_str(), &param.value))
        .collect();
    let values = cellar_sql::order_values(&prepared.parameters, &by_name)
        .map_err(|error| CellarError::query(error.to_string()))?
        .into_iter()
        .copied()
        .collect();
    Ok((prepared.sql, values))
}

fn bind_value(query: &mut TdsQuery<'static>, value: &CellValue) -> CellarResult<()> {
    match value {
        CellValue::Null => query.bind(Option::<String>::None),
        CellValue::Bool(value) => query.bind(*value),
        CellValue::Int(value) => query.bind(*value),
        CellValue::Float(value) => query.bind(*value),
        CellValue::Numeric(value) => query.bind(parse_numeric(value)?),
        CellValue::Text(value) => query.bind(value.clone()),
        CellValue::Bytes(value) => query.bind(value.clone()),
        CellValue::Json(value) => query.bind(value.to_string()),
        CellValue::Uuid(value) => query.bind(*value),
        CellValue::Date(value) => query.bind(*value),
        CellValue::Time(value) => query.bind(*value),
        CellValue::Timestamp(value) => query.bind(*value),
        CellValue::TimestampTz(value) => query.bind(*value),
    }
    Ok(())
}

fn parse_numeric(value: &str) -> CellarResult<tiberius::numeric::Numeric> {
    let value = value.trim();
    let (negative, value) = match value.strip_prefix('-') {
        Some(value) => (true, value),
        None => (false, value.strip_prefix('+').unwrap_or(value)),
    };
    let (whole, fraction) = value.split_once('.').unwrap_or((value, ""));
    if whole.is_empty() && fraction.is_empty()
        || !whole.chars().all(|c| c.is_ascii_digit())
        || !fraction.chars().all(|c| c.is_ascii_digit())
        || fraction.len() >= 38
    {
        return Err(CellarError::query(format!(
            "numeric foreign-key value `{value}` is not supported by SQL Server binding"
        )));
    }
    let digits = format!("{whole}{fraction}");
    let mut integer = digits.parse::<i128>().map_err(|_| {
        CellarError::query(format!(
            "numeric foreign-key value `{value}` is out of range"
        ))
    })?;
    if negative {
        integer = -integer;
    }
    Ok(tiberius::numeric::Numeric::new_with_scale(
        integer,
        fraction.len() as u8,
    ))
}
