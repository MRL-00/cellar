use std::time::Instant;

use cellar_core::error::{CellarError, CellarResult};
use cellar_core::query::{Query, QueryResult};
use cellar_core::value::{ColumnMeta, Row};
use futures::TryStreamExt;
use sqlx::{Column as _, Row as _, TypeInfo as _};

use crate::connect::PgConnection;
use crate::decode::decode_cell;

const DEFAULT_MAX_ROWS: u32 = 500;

pub async fn execute_query(conn: &PgConnection, query: &Query) -> CellarResult<QueryResult> {
    // Route to the pool for the query's target database so the sidebar can
    // browse and query several databases through one connection.
    let database = query
        .database
        .as_deref()
        .unwrap_or(conn.config().database.as_str());
    let pool = conn.pool_for_database(database).await?;
    let pool = &pool;

    let max_rows = query.max_rows.unwrap_or(DEFAULT_MAX_ROWS);
    let max_rows_usize = max_rows as usize;
    let capacity = max_rows.min(10_000) as usize;

    // SQL passes through verbatim. Driving LIMIT into user-supplied SQL would
    // need a parser; we cap while reading the stream instead of materializing
    // the full server result first.
    let started = Instant::now();
    let mut stream = sqlx::query(&query.sql).fetch(pool);
    let mut columns: Option<Vec<ColumnMeta>> = None;
    let mut materialized: Vec<Row> = Vec::with_capacity(capacity);
    let mut truncated = false;

    while let Some(r) = stream
        .try_next()
        .await
        .map_err(|e| CellarError::query(e.to_string()))?
    {
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

        if materialized.len() >= max_rows_usize {
            truncated = true;
            break;
        }

        let mut cells: Row = Vec::with_capacity(r.columns().len());
        for i in 0..r.columns().len() {
            cells.push(decode_cell(&r, i)?);
        }
        materialized.push(cells);
    }
    let duration_ms = started.elapsed().as_millis() as u64;

    Ok(QueryResult {
        columns: columns.unwrap_or_default(),
        rows: materialized,
        rows_affected: None,
        duration_ms,
        truncated,
    })
}
