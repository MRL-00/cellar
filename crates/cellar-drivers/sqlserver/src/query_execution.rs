use std::time::Instant;

use cellar_core::error::CellarResult;
use cellar_core::query::{NoticeCapture, QueryResult, QueryResultPage, QueryResultSummary};
use cellar_core::value::{ColumnMeta, Row};
use futures_util::TryStreamExt;
use tiberius::QueryItem;

use crate::connect::{TdsClient, map_tiberius_runtime_err};
use crate::decode::decode_cell;

pub(crate) async fn execute_sql(
    client: &mut TdsClient,
    sql: &str,
    max_rows: usize,
    offset: usize,
    started: Instant,
) -> CellarResult<QueryResult> {
    let mut rows = Vec::with_capacity(max_rows.min(10_000));
    let (columns, summary) = execute_sql_pages(
        client,
        sql,
        max_rows,
        offset,
        usize::MAX,
        started,
        &mut |page| {
            rows.extend(page.rows);
            Ok(())
        },
    )
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

pub(crate) async fn execute_sql_pages(
    client: &mut TdsClient,
    sql: &str,
    max_rows: usize,
    offset: usize,
    page_size: usize,
    started: Instant,
    on_page: &mut (dyn FnMut(QueryResultPage) -> CellarResult<()> + Send),
) -> CellarResult<(Vec<ColumnMeta>, QueryResultSummary)> {
    let mut stream = client
        .simple_query(sql)
        .await
        .map_err(|e| map_tiberius_runtime_err(e, "query execution"))?;
    let mut columns: Option<Vec<ColumnMeta>> = None;
    let page_size = page_size.max(1);
    let mut rows: Vec<Row> = Vec::with_capacity(max_rows.min(page_size).min(10_000));
    let mut truncated = false;
    // Count of data rows seen so far; used to implement the offset skip.
    let mut rows_seen: usize = 0;
    let mut rows_output: usize = 0;

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

                if rows_output >= max_rows {
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
                rows_output += 1;
                if rows.len() >= page_size {
                    emit_page(
                        columns.as_deref().unwrap_or_default(),
                        &mut rows,
                        rows_output,
                        on_page,
                    )?;
                    rows = Vec::with_capacity(max_rows.min(page_size).min(10_000));
                }
            }
            _ => {}
        }
    }

    emit_page(
        columns.as_deref().unwrap_or_default(),
        &mut rows,
        rows_output,
        on_page,
    )?;

    let columns = columns.unwrap_or_default();
    Ok((
        columns,
        QueryResultSummary {
            notices: Vec::new(),
            notice_capture: NoticeCapture::unsupported(
                "SQL Server informational messages are not exposed through the current tiberius query path.",
            ),
            // tiberius's simple_query stream only yields Metadata/Row items; the
            // DONE token's affected-row count never surfaces through QueryItem,
            // so DML row counts are unavailable on this path.
            rows_affected: None,
            duration_ms: started.elapsed().as_millis() as u64,
            truncated,
            total_rows: None,
            row_count: rows_output as u64,
        },
    ))
}

pub(crate) fn emit_page(
    columns: &[ColumnMeta],
    rows: &mut Vec<Row>,
    rows_output: usize,
    on_page: &mut (dyn FnMut(QueryResultPage) -> CellarResult<()> + Send),
) -> CellarResult<()> {
    if rows.is_empty() && (columns.is_empty() || rows_output > 0) {
        return Ok(());
    }
    on_page(QueryResultPage {
        columns: columns.to_vec(),
        offset: rows_output.saturating_sub(rows.len()) as u64,
        rows: std::mem::take(rows),
    })
}
