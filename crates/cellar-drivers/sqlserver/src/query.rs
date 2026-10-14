use cellar_core::error::{CellarError, CellarResult};
use cellar_core::query::{Query, QueryResult, QueryResultPage, QueryResultSummary};
use cellar_diff::{TableChangeRequest, TableCommitResult, build_mssql_plan};
use std::time::Instant;

use crate::connect::{
    SqlServerConnection, TdsClient, map_tiberius_runtime_err, session_invalidated,
};

#[path = "query_execution.rs"]
mod execution;
#[path = "query_params.rs"]
mod parameterized;

#[cfg(test)]
pub(super) use execution::emit_page;
pub(super) use execution::{execute_sql, execute_sql_pages};

const DEFAULT_MAX_ROWS: u32 = 500;

pub async fn execute_query(conn: &SqlServerConnection, query: &Query) -> CellarResult<QueryResult> {
    if query.read_only && !query.params.is_empty() {
        return Err(CellarError::query(
            "parameterized SQL Server read-only queries are not supported",
        ));
    }
    if query.read_only {
        return execute_read_only_query(conn, query).await;
    }
    if !query.params.is_empty() {
        return parameterized::execute_query(conn, query).await;
    }

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

pub async fn execute_query_stream(
    conn: &SqlServerConnection,
    query: &Query,
    page_size: usize,
    on_page: &mut (dyn FnMut(QueryResultPage) -> CellarResult<()> + Send),
) -> CellarResult<QueryResultSummary> {
    if query.read_only && !query.params.is_empty() {
        return Err(CellarError::query(
            "parameterized SQL Server read-only queries are not supported",
        ));
    }
    if query.read_only {
        let result = execute_read_only_query(conn, query).await?;
        let (pages, summary) = result.into_pages(page_size);
        for page in pages {
            on_page(page)?;
        }
        return Ok(summary);
    }
    if !query.params.is_empty() {
        let result = parameterized::execute_query(conn, query).await?;
        let (pages, summary) = result.into_pages(page_size);
        for page in pages {
            on_page(page)?;
        }
        return Ok(summary);
    }

    let max_rows = query.max_rows.unwrap_or(DEFAULT_MAX_ROWS) as usize;
    let offset = query.offset.unwrap_or(0) as usize;
    let started = Instant::now();
    conn.with_client(async |client| {
        if let Some(database) = query.database.as_deref() {
            if database != conn.config().database {
                let use_sql = format!("USE {}; {}", quote_ident(database), query.sql);
                return execute_sql_pages(
                    client, &use_sql, max_rows, offset, page_size, started, on_page,
                )
                .await
                .map(|(_, summary)| summary);
            }
        }
        execute_sql_pages(
            client, &query.sql, max_rows, offset, page_size, started, on_page,
        )
        .await
        .map(|(_, summary)| summary)
    })
    .await
}

/// SQL Server has no `SET TRANSACTION READ ONLY` (Postgres does). Guard AI
/// queries by (1) rejecting non-read-only / session-mutating SQL up front,
/// (2) running inside a transaction that must still be open afterward,
/// (3) requiring a successful rollback, and (4) restoring the home database
/// so a `USE` cannot leak across the shared TDS session.
async fn execute_read_only_query(
    conn: &SqlServerConnection,
    query: &Query,
) -> CellarResult<QueryResult> {
    assert_read_only_sql(&query.sql)?;

    let max_rows = query.max_rows.unwrap_or(DEFAULT_MAX_ROWS) as usize;
    let offset = query.offset.unwrap_or(0) as usize;
    let started = Instant::now();
    let home_db = conn.config().database.clone();
    let target_db = query
        .database
        .as_deref()
        .filter(|db| !db.is_empty())
        .unwrap_or(home_db.as_str());
    let switch_db = target_db != home_db.as_str();

    conn.with_client(async |client| {
        if switch_db {
            run_control(client, &format!("USE {}", quote_ident(target_db))).await?;
        }

        let outcome = run_read_only_batch(client, &query.sql, max_rows, offset, started).await;

        if switch_db {
            // Always restore — a leaked USE would point later queries at the
            // wrong database on this shared connection.
            if let Err(restore_err) =
                run_control(client, &format!("USE {}", quote_ident(&home_db))).await
            {
                // Poison the session so the contaminated client is dropped.
                return Err(match outcome {
                    Ok(_) => session_invalidated(format!(
                        "could not restore database [{home_db}] after read-only AI query: {restore_err}"
                    )),
                    Err(err) => session_invalidated(format!(
                        "{err}; also could not restore database [{home_db}]: {restore_err}"
                    )),
                });
            }
        }

        outcome
    })
    .await
}

async fn run_read_only_batch(
    client: &mut TdsClient,
    sql: &str,
    max_rows: usize,
    offset: usize,
    started: Instant,
) -> CellarResult<QueryResult> {
    let baseline = tran_count(client).await?;
    run_control(client, "BEGIN TRANSACTION").await?;

    let result = match execute_sql(client, sql, max_rows, offset, started).await {
        Ok(result) => result,
        Err(err) => {
            return Err(finish_with_rollback(client, baseline, err).await);
        }
    };

    let after = match tran_count(client).await {
        Ok(n) => n,
        Err(err) => {
            return Err(finish_with_rollback(client, baseline, err).await);
        }
    };
    // A nested COMMIT (or full ROLLBACK) drops @@TRANCOUNT below baseline+1
    // and would leave writes durable — refuse and unwind what we can.
    if after != baseline + 1 {
        return Err(finish_with_rollback(
            client,
            baseline,
            CellarError::query(
                "read-only AI query altered transaction state (COMMIT/ROLLBACK is not allowed)",
            ),
        )
        .await);
    }

    if let Err(err) = rollback_to_baseline(client, baseline).await {
        return Err(session_invalidated(format!(
            "read-only AI rollback failed after a successful query: {err}"
        )));
    }
    Ok(result)
}

/// Roll back after a failure. If cleanup itself fails, poison the session so
/// the shared client cannot keep an open transaction.
async fn finish_with_rollback(
    client: &mut TdsClient,
    baseline: i32,
    primary: CellarError,
) -> CellarError {
    match rollback_to_baseline(client, baseline).await {
        Ok(()) => primary,
        Err(cleanup) => session_invalidated(format!("{primary}; rollback also failed: {cleanup}")),
    }
}

async fn tran_count(client: &mut TdsClient) -> CellarResult<i32> {
    let row = client
        .simple_query("SELECT CONVERT(int, @@TRANCOUNT) AS tc")
        .await
        .map_err(|e| map_tiberius_runtime_err(e, "transaction state"))?
        .into_row()
        .await
        .map_err(|e| map_tiberius_runtime_err(e, "transaction state"))?
        .ok_or_else(|| CellarError::query("SQL Server did not return @@TRANCOUNT"))?;
    row.try_get::<i32, _>("tc")
        .map_err(|e| CellarError::decode(e.to_string()))?
        .ok_or_else(|| CellarError::decode("SQL Server returned NULL for @@TRANCOUNT"))
}

/// Roll back until @@TRANCOUNT is at or below `baseline`. Unlike the
/// best-effort [`rollback`] used on grid-commit errors, failures here surface
/// so a shared session cannot keep an open AI transaction.
async fn rollback_to_baseline(client: &mut TdsClient, baseline: i32) -> CellarResult<()> {
    // Unnamed ROLLBACK clears the whole stack to zero; if we nested under an
    // unexpected outer transaction, fall back to rolling until baseline.
    let current = tran_count(client).await?;
    if current <= baseline {
        return Ok(());
    }
    if baseline == 0 {
        run_control(client, "IF @@TRANCOUNT > 0 ROLLBACK TRANSACTION").await?;
    } else {
        // Should be rare on the single-client mutex path; unwind one level at
        // a time with a named savepoint-free rollback when possible.
        run_control(client, "IF @@TRANCOUNT > 0 ROLLBACK TRANSACTION").await?;
    }
    let after = tran_count(client).await?;
    if after > baseline {
        return Err(CellarError::query(format!(
            "failed to roll back read-only AI transaction (@@TRANCOUNT={after}, expected ≤ {baseline})"
        )));
    }
    if after < baseline {
        return Err(CellarError::query(format!(
            "read-only AI rollback cleared an unexpected outer transaction (@@TRANCOUNT={after}, expected {baseline})"
        )));
    }
    Ok(())
}

/// Strip comments and string literals so keyword checks cannot be fooled by
/// `'INSERT'` or `-- COMMIT` noise.
fn strip_sql_noise(sql: &str) -> String {
    let bytes = sql.as_bytes();
    let mut out = String::with_capacity(sql.len());
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        let next = bytes.get(i + 1).map(|&b| b as char);
        if c == '-' && next == Some('-') {
            i += 2;
            while i < bytes.len() && bytes[i] as char != '\n' {
                i += 1;
            }
            continue;
        }
        if c == '/' && next == Some('*') {
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] as char == '*' && bytes[i + 1] as char == '/') {
                i += 1;
            }
            i = (i + 2).min(bytes.len());
            out.push(' ');
            continue;
        }
        if c == '\'' {
            i += 1;
            while i < bytes.len() {
                if bytes[i] as char == '\'' {
                    if bytes.get(i + 1).map(|&b| b as char) == Some('\'') {
                        i += 2;
                        continue;
                    }
                    i += 1;
                    break;
                }
                i += 1;
            }
            out.push(' ');
            continue;
        }
        out.push(c);
        i += 1;
    }
    out
}

/// Backend allow-list for AI read-only runs. Transaction control, session SETs,
/// and DML/DDL are rejected because a batch can otherwise `COMMIT` out of our
/// wrapper transaction or leave lasting session side effects.
fn assert_read_only_sql(sql: &str) -> CellarResult<()> {
    let tokens = sql_tokens(&strip_sql_noise(sql));
    if tokens.is_empty() {
        return Err(CellarError::query("read-only AI query is empty"));
    }

    const ALLOWED_HEAD: &[&str] = &["select", "with"];
    if !ALLOWED_HEAD.contains(&tokens[0].as_str()) {
        return Err(CellarError::query(format!(
            "read-only AI queries must be SELECT/WITH (found leading `{}`)",
            tokens[0]
        )));
    }

    const FORBIDDEN: &[&str] = &[
        "insert",
        "update",
        "delete",
        "merge",
        "drop",
        "truncate",
        "alter",
        "create",
        "grant",
        "revoke",
        "exec",
        "execute",
        "commit",
        "rollback",
        "begin",
        "save",
        "use",
        "set",
        "into", // SELECT INTO creates a table
        "backup",
        "restore",
        "openrowset",
        "opendatasource",
        "bulk",
        // Dynamic SQL entry points — their string bodies are stripped, so we
        // must reject the procedure name itself.
        "sp_executesql",
        "sp_execute",
        "sp_prepexec",
        "sp_cursoropen",
    ];
    for (i, token) in tokens.iter().enumerate() {
        if FORBIDDEN.contains(&token.as_str()) {
            return Err(CellarError::query(format!(
                "read-only AI queries cannot contain `{token}`"
            )));
        }
        if token.starts_with("xp_") || token.starts_with("sp_exec") {
            return Err(CellarError::query(format!(
                "read-only AI queries cannot call `{token}`"
            )));
        }
        // NEXT VALUE FOR …
        if token == "next" && tokens.get(i + 1).map(String::as_str) == Some("value") {
            return Err(CellarError::query(
                "read-only AI queries cannot contain `next value`",
            ));
        }
    }
    Ok(())
}

/// Split stripped SQL into identifier tokens. Punctuation (`;`, commas, parens)
/// is a boundary so `SELECT 1;SET …` and `SELECT 1;EXEC …` still surface
/// forbidden keywords.
fn sql_tokens(sql: &str) -> Vec<String> {
    let chars: Vec<char> = sql.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        // Bracketed / quoted identifiers → one token (contents only).
        if c == '[' || c == '"' || c == '`' {
            let close = if c == '[' { ']' } else { c };
            i += 1;
            let start = i;
            while i < chars.len() && chars[i] != close {
                i += 1;
            }
            if start < i {
                tokens.push(
                    chars[start..i]
                        .iter()
                        .collect::<String>()
                        .to_ascii_lowercase(),
                );
            }
            if i < chars.len() {
                i += 1;
            }
            continue;
        }
        if c.is_ascii_alphanumeric() || c == '_' || c == '@' || c == '#' || c == '$' {
            let start = i;
            i += 1;
            while i < chars.len() {
                let n = chars[i];
                if n.is_ascii_alphanumeric() || n == '_' || n == '@' || n == '#' || n == '$' {
                    i += 1;
                } else {
                    break;
                }
            }
            tokens.push(
                chars[start..i]
                    .iter()
                    .collect::<String>()
                    .to_ascii_lowercase(),
            );
            continue;
        }
        // Punctuation is only a separator.
        i += 1;
    }
    tokens
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
                Ok(result) => {
                    let statement_rows = result.total();
                    if !statement_row_count_valid(check, statement_rows) {
                        rollback(client).await;
                        return Err(CellarError::query(format!(
                            "one table change affected {statement_rows} rows; expected {}",
                            match check {
                                RowCountCheck::Exact => "exactly one",
                                RowCountCheck::AtMost => "at most one",
                            }
                        )));
                    }
                    rows_affected += statement_rows;
                }
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

fn statement_row_count_valid(check: RowCountCheck, rows: u64) -> bool {
    match check {
        RowCountCheck::Exact => rows == 1,
        RowCountCheck::AtMost => rows <= 1,
    }
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
        .map_err(|e| map_tiberius_runtime_err(e, "control statement"))?
        .into_results()
        .await
        .map(|_| ())
        .map_err(|e| map_tiberius_runtime_err(e, "control statement"))
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

#[cfg(test)]
mod read_only_tests {
    use cellar_core::value::{CellValue, ColumnMeta};

    use super::{
        assert_read_only_sql, emit_page, sql_tokens, statement_row_count_valid, strip_sql_noise,
        RowCountCheck,
    };

    #[test]
    fn import_statements_cannot_mutate_multiple_rows() {
        assert!(statement_row_count_valid(RowCountCheck::AtMost, 0));
        assert!(statement_row_count_valid(RowCountCheck::AtMost, 1));
        assert!(!statement_row_count_valid(RowCountCheck::AtMost, 2));
    }

    #[test]
    fn emits_ordered_bounded_pages() {
        let columns = vec![ColumnMeta {
            name: "n".into(),
            data_type: "int".into(),
            nullable: false,
        }];
        let mut rows = vec![vec![CellValue::Int(3)], vec![CellValue::Int(4)]];
        let mut pages = Vec::new();
        emit_page(&columns, &mut rows, 4, &mut |page| {
            pages.push(page);
            Ok(())
        })
        .unwrap();
        assert!(rows.is_empty());
        assert_eq!(pages[0].offset, 2);
        assert_eq!(pages[0].rows.len(), 2);
    }

    #[test]
    fn emits_columns_for_an_empty_result() {
        let columns = vec![ColumnMeta {
            name: "id".into(),
            data_type: "int".into(),
            nullable: false,
        }];
        let mut rows = Vec::new();
        let mut pages = Vec::new();
        emit_page(&columns, &mut rows, 0, &mut |page| {
            pages.push(page);
            Ok(())
        })
        .unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].columns, columns);
        assert!(pages[0].rows.is_empty());
    }

    #[test]
    fn allows_plain_select_and_with() {
        assert_read_only_sql("SELECT Id FROM epiczone.Customers").unwrap();
        assert_read_only_sql("WITH x AS (SELECT 1 AS n)\nSELECT * FROM x WHERE n > 0").unwrap();
    }

    #[test]
    fn rejects_commit_escape_and_dml() {
        let err = assert_read_only_sql("SELECT 1;\nINSERT INTO t VALUES (1);\nCOMMIT TRANSACTION")
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("insert") || err.contains("commit"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn rejects_session_mutating_statements() {
        // No space after `;` — punctuation must still be a token boundary.
        assert!(assert_read_only_sql("SELECT 1;SET LOCK_TIMEOUT 1000")
            .unwrap_err()
            .to_string()
            .contains("set"));
        assert!(assert_read_only_sql("USE otherdb; SELECT 1")
            .unwrap_err()
            .to_string()
            .contains("use"));
        assert!(assert_read_only_sql("SELECT NEXT VALUE FOR dbo.seq")
            .unwrap_err()
            .to_string()
            .contains("next value"));
    }

    #[test]
    fn rejects_dynamic_sql_even_when_body_is_stripped() {
        let err = assert_read_only_sql(
            "SELECT 1;EXEC sp_executesql N'INSERT INTO dbo.t VALUES (1); COMMIT'",
        )
        .unwrap_err()
        .to_string()
        .to_ascii_lowercase();
        assert!(
            err.contains("exec") || err.contains("sp_executesql"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn ignores_forbidden_words_inside_strings_and_comments() {
        assert_read_only_sql(
            "SELECT Id FROM t -- COMMIT TRANSACTION\nWHERE note = 'please INSERT quietly'",
        )
        .unwrap();
        let stripped = strip_sql_noise("SELECT 1 -- COMMIT\nWHERE x = 'INSERT'");
        assert!(!stripped.to_ascii_lowercase().contains("commit"));
        assert!(!stripped.to_ascii_lowercase().contains("insert"));
    }

    #[test]
    fn rejects_non_select_heads() {
        assert!(assert_read_only_sql("DELETE FROM t")
            .unwrap_err()
            .to_string()
            .contains("SELECT/WITH"));
    }

    #[test]
    fn tokens_split_on_semicolons() {
        assert_eq!(
            sql_tokens("SELECT 1;SET LOCK_TIMEOUT 1000"),
            vec!["select", "1", "set", "lock_timeout", "1000"]
        );
    }
}
