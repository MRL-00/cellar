use std::borrow::Cow;
use std::collections::HashMap;
use std::time::Instant;

use cellar_core::driver::Engine;
use cellar_core::error::{CellarError, CellarResult};
use cellar_core::query::{
    NoticeCapture, PlanDetail, PlanMode, PlanNode, Query, QueryPlan, QueryResult,
};
use cellar_core::value::{CellValue, ColumnMeta, Row};
use cellar_diff::{build_postgres_plan, TableChangeRequest, TableCommitResult};
use futures::TryStreamExt;
use serde_json::{Map, Value};
use sqlx::{Column as _, Row as _, TypeInfo as _};

use crate::connect::PgConnection;
use crate::decode::decode_cell;

const DEFAULT_MAX_ROWS: u32 = 500;

/// Drop guard that removes a query from the connection's active-query
/// registry on every exit path of `execute_query`.
struct RegisteredQuery<'a> {
    conn: &'a PgConnection,
    query_id: &'a str,
}

impl Drop for RegisteredQuery<'_> {
    fn drop(&mut self) {
        self.conn.unregister_query(self.query_id);
    }
}

/// Signal the backend running `query_id` with `pg_cancel_backend`. Runs on a
/// second pool connection — the one executing the statement stays busy until
/// the server acts on the signal. Returns `false` when nothing is registered
/// under that id (the statement already finished or never started).
pub async fn cancel_query(conn: &PgConnection, query_id: &str) -> CellarResult<bool> {
    let Some(active) = conn.lookup_query(query_id) else {
        return Ok(false);
    };
    let pool = conn.pool_for_database(&active.database).await?;
    let cancelled: bool = sqlx::query_scalar("SELECT pg_cancel_backend($1)")
        .bind(active.pid)
        .fetch_one(&pool)
        .await
        .map_err(|e| {
            crate::connect::map_sqlx_err_for_runtime(e, "query cancellation", CellarError::query)
        })?;
    Ok(cancelled)
}

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
    // offset for free-form queries: skip rows from the stream. Because the SQL
    // passes through verbatim (no parser to inject OFFSET), we consume and
    // discard the leading rows. This transfers skipped rows over the wire — an
    // acceptable trade-off for small offsets (e.g. "Load more" pages of 500
    // rows each). Very large offsets would benefit from a subquery wrapper, but
    // that requires dialect-aware SQL rewriting and is deferred.
    let skip_rows = query.offset.unwrap_or(0) as usize;
    let capacity = max_rows.min(10_000) as usize;

    // Resolve parameters before building the statement. With no params the SQL
    // passes through verbatim (the existing fast path); otherwise cellar-sql
    // rewrites named/positional placeholders to `$1..$N` and reports them in
    // bind order, and we bind the typed values through sqlx — never by
    // interpolating them into the SQL text.
    let prepared_sql: Cow<str>;
    let bind_values: Vec<&CellValue>;
    if query.params.is_empty() {
        prepared_sql = Cow::Borrowed(query.sql.as_str());
        bind_values = Vec::new();
    } else {
        let prepared = cellar_sql::prepare(&query.sql, Engine::Postgres)
            .map_err(|e| CellarError::query(e.to_string()))?;
        let by_name: HashMap<&str, &CellValue> = query
            .params
            .iter()
            .map(|p| (p.name.as_str(), &p.value))
            .collect();
        bind_values = cellar_sql::order_values(&prepared.parameters, &by_name)
            .map_err(|e| CellarError::query(e.to_string()))?
            .into_iter()
            .copied()
            .collect();
        prepared_sql = Cow::Owned(prepared.sql);
    }
    let exec_sql: &str = prepared_sql.as_ref();

    // SQL passes through verbatim. Driving LIMIT into user-supplied SQL would
    // need a parser; we cap while reading the stream instead of materializing
    // the full server result first.
    let started = Instant::now();

    // When the caller set a query_id it wants cancellation support: pin the
    // statement to one pool connection so its backend PID is known up front,
    // and register it so a concurrent cancel_query can signal that PID with
    // pg_cancel_backend. The registration drops (and unregisters) on every
    // exit path, including errors.
    let mut pinned = None;
    let _registration = match query.query_id.as_deref() {
        Some(query_id) => {
            let mut acquired = pool.acquire().await.map_err(query_sqlx_err)?;
            let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
                .fetch_one(&mut *acquired)
                .await
                .map_err(query_sqlx_err)?;
            conn.register_query(query_id, pid, database);
            pinned = Some(acquired);
            Some(RegisteredQuery { conn, query_id })
        }
        None => None,
    };

    // fetch_many (vs fetch) also yields the CommandComplete arm, which carries
    // the affected-row count for DML. Deprecated in sqlx 0.8 over SQLite
    // multi-statement semantics that don't apply to a single Postgres
    // statement; the replacement (raw_sql) would switch results to the
    // text protocol and change every decode path.
    let mut statement = sqlx::query(exec_sql);
    for &value in &bind_values {
        statement = crate::bind::bind_value(statement, value)?;
    }
    #[allow(deprecated)]
    let mut stream = match pinned.as_mut() {
        Some(acquired) => statement.fetch_many(&mut **acquired),
        None => statement.fetch_many(pool),
    };
    let mut columns: Option<Vec<ColumnMeta>> = None;
    let mut materialized: Vec<Row> = Vec::with_capacity(capacity);
    let mut truncated = false;
    let mut rows_seen: usize = 0;
    let mut rows_affected: Option<u64> = None;

    while let Some(item) = stream.try_next().await.map_err(query_sqlx_err)? {
        let r = match item {
            sqlx::Either::Left(done) => {
                // Postgres reports the command tag's row count for every
                // statement, including SELECT; it only means "affected" for
                // row-returning-free statements, which is gated below.
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

        // Skip rows before the requested offset.
        if rows_seen < skip_rows {
            rows_seen += 1;
            continue;
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

    // Only surface an affected count for statements without a result set
    // (INSERT/UPDATE/DELETE/DDL without RETURNING). For row-returning
    // statements the tag count is just the row count and the UI would
    // mislabel a SELECT as "N rows affected".
    let rows_affected = if columns.is_none() {
        rows_affected
    } else {
        None
    };

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
        rows_affected,
        duration_ms,
        truncated,
        // Total row count is not available for free-form queries without
        // wrapping the SQL in a COUNT subquery, which requires parsing.
        total_rows: None,
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
    let mut tx = pool.begin().await.map_err(query_sqlx_err)?;
    let mut rows_affected = 0u64;

    for statement in &plan.statements {
        let result = sqlx::query(statement)
            .execute(&mut *tx)
            .await
            .map_err(query_sqlx_err)?;
        rows_affected += result.rows_affected();
    }

    if rows_affected != plan.preview.expected_rows {
        tx.rollback().await.map_err(query_sqlx_err)?;
        return Err(CellarError::query(format!(
            "expected {} affected rows but database reported {}; the table may have changed since it was loaded",
            plan.preview.expected_rows, rows_affected
        )));
    }

    tx.commit().await.map_err(query_sqlx_err)?;

    Ok(TableCommitResult {
        sql: plan.preview.sql,
        rows_affected,
        duration_ms: started.elapsed().as_millis() as u64,
    })
}

/// Apply a (user-reviewed, possibly edited) schema migration script against
/// `database`. The script runs through the simple query protocol so multiple
/// statements — and any `BEGIN;`/`COMMIT;` the generated script wraps them in —
/// are honored as written. If a statement fails inside the wrapping
/// transaction, Postgres aborts it and the whole script rolls back. Returns
/// the elapsed time in milliseconds.
pub async fn apply_migration(conn: &PgConnection, database: &str, sql: &str) -> CellarResult<u64> {
    let pool = conn.pool_for_database(database).await?;
    let started = Instant::now();
    sqlx::raw_sql(sql)
        .execute(&pool)
        .await
        .map_err(query_sqlx_err)?;
    Ok(started.elapsed().as_millis() as u64)
}

pub async fn explain_query(
    conn: &PgConnection,
    query: &Query,
    mode: PlanMode,
) -> CellarResult<QueryPlan> {
    let database = query
        .database
        .as_deref()
        .unwrap_or(conn.config().database.as_str());
    let pool = conn.pool_for_database(database).await?;
    let statement = normalize_single_statement(&query.sql)?;
    let explain_sql = match mode {
        PlanMode::Estimate => format!("EXPLAIN (FORMAT JSON) {statement}"),
        PlanMode::Analyze => format!("EXPLAIN (ANALYZE, FORMAT JSON) {statement}"),
    };

    let started = Instant::now();
    let raw_json: Value = sqlx::query_scalar(&explain_sql)
        .fetch_one(&pool)
        .await
        .map_err(query_sqlx_err)?;
    let duration_ms = started.elapsed().as_millis() as u64;

    let root_doc = raw_json
        .as_array()
        .and_then(|a| a.first())
        .and_then(Value::as_object)
        .ok_or_else(|| CellarError::decode("Postgres returned an unexpected EXPLAIN JSON shape"))?;
    let root = root_doc
        .get("Plan")
        .ok_or_else(|| CellarError::decode("Postgres EXPLAIN JSON did not include a Plan"))?;

    Ok(QueryPlan {
        mode,
        engine: "postgres".into(),
        database: Some(database.to_string()),
        sql: statement,
        root: parse_plan_node(root)?,
        planning_time_ms: f64_field(root_doc, "Planning Time"),
        execution_time_ms: f64_field(root_doc, "Execution Time"),
        duration_ms,
        raw_json,
    })
}

fn parse_plan_node(value: &Value) -> CellarResult<PlanNode> {
    let obj = value
        .as_object()
        .ok_or_else(|| CellarError::decode("Postgres plan node was not an object"))?;
    let children = obj
        .get("Plans")
        .and_then(Value::as_array)
        .map(|plans| {
            plans
                .iter()
                .map(parse_plan_node)
                .collect::<CellarResult<Vec<_>>>()
        })
        .transpose()?
        .unwrap_or_default();

    Ok(PlanNode {
        node_type: string_field(obj, "Node Type").unwrap_or_else(|| "Unknown".into()),
        relation_name: string_field(obj, "Relation Name"),
        schema_name: string_field(obj, "Schema"),
        alias: string_field(obj, "Alias"),
        index_name: string_field(obj, "Index Name"),
        join_type: string_field(obj, "Join Type"),
        startup_cost: f64_field(obj, "Startup Cost"),
        total_cost: f64_field(obj, "Total Cost"),
        plan_rows: u64_field(obj, "Plan Rows"),
        plan_width: u64_field(obj, "Plan Width"),
        actual_startup_time_ms: f64_field(obj, "Actual Startup Time"),
        actual_total_time_ms: f64_field(obj, "Actual Total Time"),
        actual_rows: f64_field(obj, "Actual Rows"),
        actual_loops: u64_field(obj, "Actual Loops"),
        details: detail_fields(obj),
        children,
    })
}

fn string_field(obj: &Map<String, Value>, key: &str) -> Option<String> {
    obj.get(key).and_then(Value::as_str).map(ToOwned::to_owned)
}

fn f64_field(obj: &Map<String, Value>, key: &str) -> Option<f64> {
    obj.get(key).and_then(Value::as_f64)
}

fn u64_field(obj: &Map<String, Value>, key: &str) -> Option<u64> {
    obj.get(key).and_then(Value::as_u64)
}

fn detail_fields(obj: &Map<String, Value>) -> Vec<PlanDetail> {
    const STRUCTURAL: &[&str] = &[
        "Node Type",
        "Relation Name",
        "Schema",
        "Alias",
        "Index Name",
        "Join Type",
        "Startup Cost",
        "Total Cost",
        "Plan Rows",
        "Plan Width",
        "Actual Startup Time",
        "Actual Total Time",
        "Actual Rows",
        "Actual Loops",
        "Plans",
        "Parent Relationship",
        "Parallel Aware",
        "Async Capable",
    ];
    let mut details: Vec<_> = obj
        .iter()
        .filter(|(k, _)| !STRUCTURAL.contains(&k.as_str()))
        .filter_map(|(label, value)| {
            format_detail(value).map(|value| PlanDetail {
                label: label.clone(),
                value,
            })
        })
        .collect();
    details.sort_by(|a, b| a.label.cmp(&b.label));
    details
}

fn format_detail(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::Bool(v) => Some(v.to_string()),
        Value::Number(v) => Some(v.to_string()),
        Value::String(v) => Some(v.clone()),
        Value::Array(values) => Some(
            values
                .iter()
                .filter_map(format_detail)
                .collect::<Vec<_>>()
                .join(", "),
        ),
        Value::Object(_) => Some(value.to_string()),
    }
}

fn normalize_single_statement(sql: &str) -> CellarResult<String> {
    let trimmed = sql.trim();
    if trimmed.is_empty() {
        return Err(CellarError::query("cannot explain an empty SQL statement"));
    }

    if let Some(idx) = first_top_level_semicolon(trimmed) {
        let after = &trimmed[idx + 1..];
        if has_sql_tokens(after) {
            return Err(CellarError::query(
                "EXPLAIN only accepts one statement; run statements separately",
            ));
        }
        Ok(trimmed[..idx].trim().to_string())
    } else {
        Ok(trimmed.to_string())
    }
}

fn query_sqlx_err(err: sqlx::Error) -> CellarError {
    if let sqlx::Error::Database(db) = &err {
        // SQLSTATE 57014 query_canceled — raised by pg_cancel_backend and by
        // statement_timeout. The server message ("canceling statement due to
        // user request") reads better than the generic sqlx wrapper.
        if db.code().as_deref() == Some("57014") {
            return CellarError::Query(db.message().to_string());
        }
    }
    crate::connect::map_sqlx_err_for_runtime(err, "query execution", CellarError::query)
}

fn has_sql_tokens(sql: &str) -> bool {
    for (idx, ch) in sql.char_indices() {
        if !ch.is_whitespace() {
            if sql[idx..].starts_with("--") {
                return sql[idx..]
                    .find('\n')
                    .is_some_and(|line_end| has_sql_tokens(&sql[idx + line_end + 1..]));
            }
            if sql[idx..].starts_with("/*") {
                return sql[idx + 2..]
                    .find("*/")
                    .is_none_or(|end| has_sql_tokens(&sql[idx + end + 4..]));
            }
            return true;
        }
    }
    false
}

fn first_top_level_semicolon(sql: &str) -> Option<usize> {
    let bytes = sql.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\'' => i = skip_single_quote(bytes, i + 1),
            b'"' => i = skip_double_quote(bytes, i + 1),
            b'-' if bytes.get(i + 1) == Some(&b'-') => i = skip_line_comment(bytes, i + 2),
            b'/' if bytes.get(i + 1) == Some(&b'*') => i = skip_block_comment(bytes, i + 2),
            b'$' => {
                if let Some(next) = skip_dollar_quote(sql, i) {
                    i = next;
                } else {
                    i += 1;
                }
            }
            b';' => return Some(i),
            _ => i += 1,
        }
    }
    None
}

fn skip_single_quote(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() {
        if bytes[i] == b'\'' {
            if bytes.get(i + 1) == Some(&b'\'') {
                i += 2;
            } else {
                return i + 1;
            }
        } else {
            i += 1;
        }
    }
    i
}

fn skip_double_quote(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() {
        if bytes[i] == b'"' {
            if bytes.get(i + 1) == Some(&b'"') {
                i += 2;
            } else {
                return i + 1;
            }
        } else {
            i += 1;
        }
    }
    i
}

fn skip_line_comment(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && bytes[i] != b'\n' {
        i += 1;
    }
    i
}

fn skip_block_comment(bytes: &[u8], mut i: usize) -> usize {
    let mut depth = 1;
    while i + 1 < bytes.len() {
        if bytes[i] == b'/' && bytes[i + 1] == b'*' {
            depth += 1;
            i += 2;
        } else if bytes[i] == b'*' && bytes[i + 1] == b'/' {
            depth -= 1;
            i += 2;
            if depth == 0 {
                return i;
            }
        } else {
            i += 1;
        }
    }
    bytes.len()
}

fn skip_dollar_quote(sql: &str, start: usize) -> Option<usize> {
    let bytes = sql.as_bytes();
    let mut end = start + 1;
    while end < bytes.len() {
        let b = bytes[end];
        if b == b'$' {
            let tag = &sql[start..=end];
            return sql[end + 1..]
                .find(tag)
                .map(|close| end + 1 + close + tag.len());
        }
        if !(b == b'_' || b.is_ascii_alphanumeric()) {
            return None;
        }
        end += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn normalizes_a_single_statement_with_trailing_comments() {
        let sql = " SELECT ';' AS semi; -- ok\n /* done */ ";
        assert_eq!(
            normalize_single_statement(sql).expect("single statement"),
            "SELECT ';' AS semi"
        );
    }

    #[test]
    fn rejects_multiple_statements() {
        let err = normalize_single_statement("SELECT 1; DROP TABLE users")
            .expect_err("multiple statements rejected");
        assert!(err.to_string().contains("one statement"));
    }

    #[test]
    fn ignores_semicolons_in_dollar_quotes() {
        let sql = "SELECT $$semi;colon$$ AS body";
        assert_eq!(normalize_single_statement(sql).unwrap(), sql);
    }

    #[test]
    fn parses_json_plan_nodes() {
        let plan = json!({
            "Node Type": "Seq Scan",
            "Relation Name": "orders",
            "Startup Cost": 0.0,
            "Total Cost": 12.5,
            "Plan Rows": 10,
            "Filter": "(total > 10)"
        });
        let parsed = parse_plan_node(&plan).expect("parse plan");
        assert_eq!(parsed.node_type, "Seq Scan");
        assert_eq!(parsed.relation_name.as_deref(), Some("orders"));
        assert_eq!(parsed.details[0].label, "Filter");
    }
}
