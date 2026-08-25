use std::time::Instant;

use cellar_core::error::CellarResult;
use cellar_core::query::{NoticeCapture, Query, QueryResult, QueryResultPage, QueryResultSummary};
use cellar_core::value::{ColumnMeta, Row};
use futures::TryStreamExt;
use sqlx::{Column as _, Executor as _, Row as _, TypeInfo as _};

use crate::connect::MySqlConnection;
use crate::decode::decode_cell;

const DEFAULT_MAX_ROWS: u32 = 500;

pub async fn execute_query(conn: &MySqlConnection, query: &Query) -> CellarResult<QueryResult> {
    let mut rows =
        Vec::with_capacity(query.max_rows.unwrap_or(DEFAULT_MAX_ROWS).min(10_000) as usize);
    let (columns, summary) = execute_query_pages(conn, query, usize::MAX, |page| {
        rows.extend(page.rows);
        Ok(())
    })
    .await?;
    Ok(QueryResult {
        columns,
        rows,
        notices: summary.notices,
        notice_capture: summary.notice_capture,
        rows_affected: summary.rows_affected,
        duration_ms: summary.duration_ms,
        truncated: summary.truncated,
        total_rows: summary.total_rows,
    })
}

pub async fn execute_query_stream(
    conn: &MySqlConnection,
    query: &Query,
    page_size: usize,
    on_page: &mut (dyn FnMut(QueryResultPage) -> CellarResult<()> + Send),
) -> CellarResult<QueryResultSummary> {
    execute_query_pages(conn, query, page_size, on_page)
        .await
        .map(|(_, summary)| summary)
}

async fn execute_query_pages<F>(
    conn: &MySqlConnection,
    query: &Query,
    page_size: usize,
    mut on_page: F,
) -> CellarResult<(Vec<ColumnMeta>, QueryResultSummary)>
where
    F: FnMut(QueryResultPage) -> CellarResult<()> + Send,
{
    let pool = conn.pool();
    let max_rows = query.max_rows.unwrap_or(DEFAULT_MAX_ROWS) as usize;
    let offset = query.offset.unwrap_or(0) as usize;
    let page_size = page_size.max(1);
    let started = Instant::now();
    let mut acquired = pool.acquire().await.map_err(query_sqlx_err)?;

    // fetch_many (vs fetch) also yields the command-complete arm, which carries
    // the affected-row count for DML (INSERT/UPDATE/DELETE). fetch alone drops
    // it, so those statements would never report a count.
    #[allow(deprecated)]
    let mut stream = sqlx::query(&query.sql).fetch_many(&mut *acquired);
    let mut columns: Option<Vec<ColumnMeta>> = None;
    let mut page_rows: Vec<Row> = Vec::with_capacity(max_rows.min(page_size).min(10_000));
    let mut truncated = false;
    let mut rows_seen: usize = 0;
    let mut rows_output: usize = 0;
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
        if rows_output >= max_rows {
            truncated = true;
            break;
        }

        let mut cells: Row = Vec::with_capacity(r.columns().len());
        for i in 0..r.columns().len() {
            cells.push(decode_cell(&r, i)?);
        }
        page_rows.push(cells);
        rows_output += 1;
        if page_rows.len() >= page_size {
            on_page(QueryResultPage {
                columns: columns.clone().unwrap_or_default(),
                rows: std::mem::take(&mut page_rows),
                offset: (rows_output - page_size) as u64,
            })?;
            page_rows = Vec::with_capacity(max_rows.min(page_size).min(10_000));
        }
    }
    drop(stream);

    if rows_output == 0
        && columns.is_none()
        && cellar_core::query::statement_may_return_rows(&query.sql)
    {
        let described = (&mut *acquired)
            .describe(&query.sql)
            .await
            .map_err(query_sqlx_err)?;
        if !described.columns().is_empty() {
            columns = Some(
                described
                    .columns()
                    .iter()
                    .map(|c| ColumnMeta {
                        name: c.name().to_string(),
                        data_type: c.type_info().name().to_string().to_lowercase(),
                        nullable: true,
                    })
                    .collect(),
            );
        }
    }

    if !page_rows.is_empty() {
        let page_offset = rows_output - page_rows.len();
        on_page(QueryResultPage {
            columns: columns.clone().unwrap_or_default(),
            rows: page_rows,
            offset: page_offset as u64,
        })?;
    } else if rows_output == 0 && columns.is_some() {
        on_page(QueryResultPage {
            columns: columns.clone().unwrap_or_default(),
            rows: Vec::new(),
            offset: 0,
        })?;
    }

    // Only surface an affected count for statements without a result set
    // (INSERT/UPDATE/DELETE/DDL). For SELECTs the count is just the row count
    // and the UI would mislabel it as "N rows affected".
    let rows_affected = if columns.is_none() {
        rows_affected
    } else {
        None
    };

    let columns = columns.unwrap_or_default();
    Ok((
        columns,
        QueryResultSummary {
            notices: Vec::new(),
            notice_capture: NoticeCapture::unsupported(
                "MySQL does not expose server notices through the sqlx query path.",
            ),
            rows_affected,
            duration_ms: started.elapsed().as_millis() as u64,
            truncated,
            total_rows: None,
            row_count: rows_output as u64,
        },
    ))
}

fn query_sqlx_err(err: sqlx::Error) -> cellar_core::error::CellarError {
    crate::connect::map_sqlx_err_for_runtime(
        err,
        "query execution",
        cellar_core::error::CellarError::query,
    )
}
