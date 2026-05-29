use std::time::Instant;

use cellar_core::error::{CellarError, CellarResult};
use cellar_core::query::{NoticeCapture, Query, QueryResult};
use cellar_core::value::{ColumnMeta, Row};
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
    // SQL passes through verbatim. Driving LIMIT into user-supplied SQL would
    // need a parser; we cap on the host side instead and signal truncation.
    let started = Instant::now();
    let rows = sqlx::query(&query.sql)
        .fetch_all(pool)
        .await
        .map_err(|e| CellarError::query(e.to_string()))?;
    let duration_ms = started.elapsed().as_millis() as u64;

    let total = rows.len();
    let capped = total.min(max_rows as usize);
    let truncated = total > capped;

    let columns = if let Some(first) = rows.first() {
        first
            .columns()
            .iter()
            .map(|c| ColumnMeta {
                name: c.name().to_string(),
                data_type: c.type_info().name().to_string().to_lowercase(),
                nullable: true,
            })
            .collect()
    } else {
        Vec::new()
    };

    let mut materialized: Vec<Row> = Vec::with_capacity(capped);
    for r in rows.iter().take(capped) {
        let mut cells: Row = Vec::with_capacity(r.columns().len());
        for i in 0..r.columns().len() {
            cells.push(decode_cell(r, i)?);
        }
        materialized.push(cells);
    }

    Ok(QueryResult {
        columns,
        rows: materialized,
        notices: Vec::new(),
        // SQLx decodes Postgres NoticeResponse frames, but PgPool consumes
        // them inside the connection stream and only emits a log/tracing
        // message. That path drops SQLSTATE/detail/hint and has no query
        // correlation hook, so Cellar reports the gap instead of pretending
        // RAISE NOTICE is captured.
        notice_capture: NoticeCapture::unsupported(
            "Postgres server notices are parsed by sqlx, but the current PgPool query path consumes NoticeResponse frames internally and exposes only log/tracing output without SQLSTATE, detail, hint, or query correlation.",
        ),
        rows_affected: None,
        duration_ms,
        truncated,
    })
}
