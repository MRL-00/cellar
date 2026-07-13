use std::time::Instant;

use cellar_core::error::{CellarError, CellarResult};
use cellar_core::query::{NoticeCapture, Query, QueryResult};
use cellar_core::value::{ColumnMeta, Row};
use cellar_diff::{build_mssql_plan, TableChangeRequest, TableCommitResult};
use futures_util::TryStreamExt;
use tiberius::QueryItem;

use crate::connect::{map_tiberius_runtime_err, SqlServerConnection, TdsClient};
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
        // tiberius's simple_query stream only yields Metadata/Row items; the
        // DONE token's affected-row count never surfaces through QueryItem,
        // so DML row counts are unavailable on this path.
        rows_affected: None,
        duration_ms: started.elapsed().as_millis() as u64,
        truncated,
        total_rows: None,
    })
}

fn quote_ident(ident: &str) -> String {
    format!("[{}]", ident.replace(']', "]]"))
}

/// How the committed row count is checked against the plan's expectation.
/// Mirrors the Postgres driver: `Exact` for grid edits, `AtMost` for
/// idempotent CSV imports whose no-op rows affect zero rows.
#[derive(Clone, Copy)]
enum RowCountCheck {
    Exact,
    AtMost,
}

pub async fn commit_table_changes(
    conn: &SqlServerConnection,
    request: &TableChangeRequest,
) -> CellarResult<TableCommitResult> {
    commit_plan(conn, request, RowCountCheck::Exact).await
}

pub async fn commit_table_import(
    conn: &SqlServerConnection,
    request: &TableChangeRequest,
) -> CellarResult<TableCommitResult> {
    commit_plan(conn, request, RowCountCheck::AtMost).await
}

async fn commit_plan(
    conn: &SqlServerConnection,
    request: &TableChangeRequest,
    check: RowCountCheck,
) -> CellarResult<TableCommitResult> {
    let plan = build_mssql_plan(request).map_err(|e| CellarError::query(e.to_string()))?;
    let started = Instant::now();

    conn.with_client(async |client| {
        // Route to the request's target database; the shared client stays on
        // the configured default otherwise.
        if let Some(database) = request.database.as_deref() {
            if database != conn.config().database {
                run_control(client, &format!("USE {}", quote_ident(database))).await?;
            }
        }

        // XACT_ABORT makes any statement error abort and roll back the whole
        // transaction server-side; the guarded ROLLBACK below covers errors
        // XACT_ABORT does not (e.g. the row-count mismatch raised client-side).
        run_control(client, "SET XACT_ABORT ON; BEGIN TRANSACTION").await?;

        let mut rows_affected = 0u64;
        for statement in &plan.statements {
            match client.execute(statement.as_str(), &[]).await {
                Ok(result) => rows_affected += result.total(),
                Err(err) => {
                    rollback(client).await;
                    return Err(map_tiberius_runtime_err(err, "table commit"));
                }
            }
        }

        let mismatch = match check {
            RowCountCheck::Exact => rows_affected != plan.preview.expected_rows,
            RowCountCheck::AtMost => rows_affected > plan.preview.expected_rows,
        };
        if mismatch {
            rollback(client).await;
            return Err(CellarError::query(format!(
                "expected {} affected rows but database reported {}; the table may have changed since it was loaded",
                plan.preview.expected_rows, rows_affected
            )));
        }

        run_control(client, "COMMIT TRANSACTION").await?;
        Ok(TableCommitResult {
            sql: plan.preview.sql.clone(),
            rows_affected,
            duration_ms: started.elapsed().as_millis() as u64,
        })
    })
    .await
}

/// Control statements (USE, BEGIN/COMMIT/ROLLBACK TRANSACTION) must run as a
/// raw batch via `simple_query`, never `Client::execute`: tiberius `execute`
/// wraps the SQL in `sp_executesql`, where changing @@TRANCOUNT raises error
/// 266 ("mismatched number of BEGIN and COMMIT statements") — which failed
/// every SQL Server grid commit — and where USE does not outlive the call.
async fn run_control(client: &mut TdsClient, sql: &str) -> CellarResult<()> {
    client
        .simple_query(sql)
        .await
        .map_err(|e| map_tiberius_runtime_err(e, "table commit"))?
        .into_results()
        .await
        .map(|_| ())
        .map_err(|e| map_tiberius_runtime_err(e, "table commit"))
}

/// Best-effort rollback on the error path; the original error is what the
/// caller needs to see, and XACT_ABORT may already have rolled back.
async fn rollback(client: &mut TdsClient) {
    if let Ok(stream) = client
        .simple_query("IF @@TRANCOUNT > 0 ROLLBACK TRANSACTION")
        .await
    {
        let _ = stream.into_results().await;
    }
}
