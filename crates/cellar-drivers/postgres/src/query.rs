use std::time::Instant;

use cellar_core::error::{CellarError, CellarResult};
use cellar_core::query::{NoticeCapture, Query, QueryResult};
use cellar_core::value::{ColumnMeta, Row};
use cellar_diff::{build_postgres_plan, TableChangeRequest, TableCommitResult};
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

pub async fn commit_table_changes(
    conn: &PgConnection,
    request: &TableChangeRequest,
) -> CellarResult<TableCommitResult> {
    let database = request
        .database
        .as_deref()
        .unwrap_or(conn.config().database.as_str());
    let pool = conn.pool_for_database(database).await?;
    let plan = build_postgres_plan(request).map_err(|e| CellarError::query(e.to_string()))?;

    let started = Instant::now();
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| CellarError::query(e.to_string()))?;
    let mut rows_affected = 0u64;

    for statement in &plan.statements {
        let result = sqlx::query(statement)
            .execute(&mut *tx)
            .await
            .map_err(|e| CellarError::query(e.to_string()))?;
        rows_affected += result.rows_affected();
    }

    if rows_affected != plan.preview.expected_rows {
        tx.rollback()
            .await
            .map_err(|e| CellarError::query(e.to_string()))?;
        return Err(CellarError::query(format!(
            "expected {} affected rows but database reported {}; the table may have changed since it was loaded",
            plan.preview.expected_rows, rows_affected
        )));
    }

    tx.commit()
        .await
        .map_err(|e| CellarError::query(e.to_string()))?;

    Ok(TableCommitResult {
        sql: plan.preview.sql,
        rows_affected,
        duration_ms: started.elapsed().as_millis() as u64,
    })
}
