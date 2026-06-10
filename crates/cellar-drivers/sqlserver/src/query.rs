use std::time::Instant;

use cellar_core::error::CellarResult;
use cellar_core::query::{NoticeCapture, Query, QueryResult};
use cellar_core::value::{ColumnMeta, Row};
use futures_util::TryStreamExt;
use tiberius::QueryItem;

use crate::connect::{map_tiberius_runtime_err, SqlServerConnection};
use crate::decode::decode_cell;

const DEFAULT_MAX_ROWS: u32 = 500;

pub async fn execute_query(conn: &SqlServerConnection, query: &Query) -> CellarResult<QueryResult> {
    let max_rows = query.max_rows.unwrap_or(DEFAULT_MAX_ROWS) as usize;
    let offset = query.offset.unwrap_or(0) as usize;
    let started = Instant::now();

    conn.with_client(async |client| {
        if let Some(database) = query.database.as_deref() {
            if database != conn.config().database {
                let use_sql = format!("USE {}; {}", quote_ident(database), query.sql);
                return execute_sql(client, &use_sql, max_rows, offset, started).await;
            }
        }
        execute_sql(client, &query.sql, max_rows, offset, started).await
    })
    .await
}

async fn execute_sql(
    client: &mut crate::connect::TdsClient,
    sql: &str,
    max_rows: usize,
    offset: usize,
    started: Instant,
) -> CellarResult<QueryResult> {
    let mut stream = client
        .simple_query(sql)
        .await
        .map_err(|e| map_tiberius_runtime_err(e, "query execution"))?;
    let mut columns: Option<Vec<ColumnMeta>> = None;
    let mut rows: Vec<Row> = Vec::with_capacity(max_rows.min(10_000));
    let mut truncated = false;
    // Count of data rows seen so far; used to implement the offset skip.
    let mut rows_seen: usize = 0;

    while let Some(item) = stream
        .try_next()
        .await
        .map_err(|e| map_tiberius_runtime_err(e, "query execution"))?
    {
        match item {
            QueryItem::Metadata(meta) if columns.is_none() => {
                columns = Some(
                    meta.columns()
                        .iter()
                        .map(|c| ColumnMeta {
                            name: c.name().to_string(),
                            data_type: format!("{:?}", c.column_type()).to_lowercase(),
                            nullable: true,
                        })
                        .collect(),
                );
            }
            QueryItem::Row(row) => {
                // Skip leading rows to honour the caller's page offset.
                // Like the Postgres driver this transfers the skipped rows
                // over the wire (no server-side OFFSET injection because the
                // SQL passes through verbatim). Acceptable for the "Load more"
                // UX where offsets are small relative to max_rows.
                if rows_seen < offset {
                    rows_seen += 1;
                    continue;
                }
                rows_seen += 1;

                if rows.len() >= max_rows {
                    truncated = true;
                    // B9 fix: break instead of continue. The previous `continue`
                    // iterated the stream to completion, transferring every
                    // remaining row over the network while decoding was skipped.
                    // Tiberius holds the TDS client in a Mutex-guarded connection;
                    // the stream borrows it mutably and is dropped here, which
                    // causes tiberius to cancel the query on the server side via
                    // the attention packet. No manual drain is necessary.
                    break;
                }
                rows.push(row.into_iter().map(decode_cell).collect());
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

fn quote_ident(ident: &str) -> String {
    format!("[{}]", ident.replace(']', "]]"))
}
