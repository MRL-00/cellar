use std::time::Instant;

use cellar_core::error::CellarResult;
use cellar_core::query::{NoticeCapture, Query, QueryResult};
use cellar_core::value::{ColumnMeta, Row};
use futures::TryStreamExt;
use sqlx::{Column as _, Row as _, TypeInfo as _};

use crate::connect::SqliteConnection;
use crate::decode::decode_cell;

const DEFAULT_MAX_ROWS: u32 = 500;

pub async fn execute_query(conn: &SqliteConnection, query: &Query) -> CellarResult<QueryResult> {
    let pool = conn.pool();
    let max_rows = query.max_rows.unwrap_or(DEFAULT_MAX_ROWS) as usize;
    let offset = query.offset.unwrap_or(0) as usize;
    let started = Instant::now();

    // fetch_many (vs fetch) also yields the command-complete arm, which carries
    // the affected-row count for DML (INSERT/UPDATE/DELETE). fetch alone drops
    // it, so those statements would never report a count.
    #[allow(deprecated)]
    let mut stream = sqlx::query(&query.sql).fetch_many(pool);
    let mut columns: Option<Vec<ColumnMeta>> = None;
    let mut rows: Vec<Row> = Vec::with_capacity(max_rows.min(10_000));
    let mut truncated = false;
    let mut rows_seen: usize = 0;
    let mut rows_affected: Option<u64> = None;

    while let Some(item) = stream.try_next().await.map_err(query_sqlx_err)? {
        let r = match item {
            sqlx::Either::Left(done) => {
                rows_affected = Some(rows_affected.unwrap_or(0) + done.rows_affected());
                continue;
            }
            sqlx::Either::Right(row) => row,
        };

        if columns.is_none() {
            columns = Some(
                r.columns()
                    .iter()
                    .map(|c| ColumnMeta {
                        name: c.name().to_string(),
                        data_type: c.type_info().name().to_string().to_lowercase(),
                        nullable: true,
                    })
                    .collect(),
            );
        }

        if rows_seen < offset {
            rows_seen += 1;
            continue;
        }
        rows_seen += 1;

        if rows.len() >= max_rows {
            truncated = true;
            break;
        }

        let mut cells: Row = Vec::with_capacity(r.columns().len());
        for i in 0..r.columns().len() {
            cells.push(decode_cell(&r, i)?);
        }
        rows.push(cells);
    }

    // Only surface an affected count for statements without a result set
    // (INSERT/UPDATE/DELETE/DDL). For SELECTs the count is just the row count
    // and the UI would mislabel it as "N rows affected".
    let rows_affected = if columns.is_none() {
        rows_affected
    } else {
        None
    };

    Ok(QueryResult {
        columns: columns.unwrap_or_default(),
        rows,
        notices: Vec::new(),
        notice_capture: NoticeCapture::unsupported("SQLite does not emit server notices."),
        rows_affected,
        duration_ms: started.elapsed().as_millis() as u64,
        truncated,
        total_rows: None,
    })
}

fn query_sqlx_err(err: sqlx::Error) -> cellar_core::error::CellarError {
    crate::connect::map_sqlx_err_for_runtime(
        err,
        "query execution",
        cellar_core::error::CellarError::query,
    )
}
