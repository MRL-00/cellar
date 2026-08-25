use std::borrow::Cow;
use std::collections::HashMap;
use std::time::Instant;

use cellar_core::driver::Engine;
use cellar_core::error::{CellarError, CellarResult};
use cellar_core::query::{
    NoticeCapture, PlanDetail, PlanMode, PlanNode, Query, QueryPlan, QueryResult, QueryResultPage,
    QueryResultSummary,
};
use cellar_core::value::{CellValue, ColumnMeta, Row};
use cellar_diff::{build_postgres_plan, TableChangeRequest, TableCommitResult};
use futures::TryStreamExt;
use serde_json::{Map, Value};
use sqlx::{Column as _, Executor as _, PgPool, Row as _, Statement as _, TypeInfo as _};

use crate::connect::PgConnection;
use crate::decode::decode_cell;
use crate::runtime::{query_sqlx_err, RegisteredQuery};

const DEFAULT_MAX_ROWS: u32 = 500;

pub async fn execute_query(conn: &PgConnection, query: &Query) -> CellarResult<QueryResult> {
    if query.read_only {
        let database = query
            .database
            .as_deref()
            .unwrap_or(conn.config().database.as_str());
        let pool = conn.pool_for_database(database).await?;
        return execute_read_only_query(&pool, query).await;
    }

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

/// Decode and deliver Postgres rows while the database cursor is active. This
/// is the hot path used by the desktop result grid; unlike `execute_query`, it
/// never needs to retain the full bounded result in Rust.
pub async fn execute_query_stream(
    conn: &PgConnection,
    query: &Query,
    page_size: usize,
    on_page: &mut (dyn FnMut(QueryResultPage) -> CellarResult<()> + Send),
) -> CellarResult<QueryResultSummary> {
    if query.read_only {
        let database = query
            .database
            .as_deref()
            .unwrap_or(conn.config().database.as_str());
        let pool = conn.pool_for_database(database).await?;
        let (pages, summary) = execute_read_only_query(&pool, query)
            .await?
            .into_pages(page_size);
        for page in pages {
            on_page(page)?;
        }
        return Ok(summary);
    }

    execute_query_pages(conn, query, page_size, on_page)
        .await
        .map(|(_, summary)| summary)
}

async fn execute_query_pages<F>(
    conn: &PgConnection,
    query: &Query,
    page_size: usize,
    mut on_page: F,
) -> CellarResult<(Vec<ColumnMeta>, QueryResultSummary)>
where
    F: FnMut(QueryResultPage) -> CellarResult<()> + Send,
{
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
    let page_size = page_size.max(1);
    // offset for free-form queries: skip rows from the stream. Because the SQL
    // passes through verbatim (no parser to inject OFFSET), we consume and
    // discard the leading rows. This transfers skipped rows over the wire — an
    // acceptable trade-off for small offsets (e.g. "Load more" pages of 500
    // rows each). Very large offsets would benefit from a subquery wrapper, but
    // that requires dialect-aware SQL rewriting and is deferred.
    let skip_rows = query.offset.unwrap_or(0) as usize;
    let capacity = max_rows_usize.min(page_size).min(10_000);

    // Resolve parameters before building the statement. With no params the SQL
    // passes through verbatim (the existing fast path); otherwise cellar-sql
    // rewrites named/positional placeholders to `$1..$N` and reports them in
    // bind order, and we bind the typed values through sqlx — never by
    // interpolating them into the SQL text.
    let (prepared_sql, bind_values) = prepare_query(query)?;
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
    let mut acquired = pool.acquire().await.map_err(query_sqlx_err)?;
    let _registration = match query.query_id.as_deref() {
        Some(query_id) => {
            let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
                .fetch_one(&mut *acquired)
                .await
                .map_err(query_sqlx_err)?;
            conn.register_query(query_id, pid, database);
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
    let mut stream = statement.fetch_many(&mut *acquired);
    let mut columns: Option<Vec<ColumnMeta>> = None;
    let mut page_rows: Vec<Row> = Vec::with_capacity(capacity);
    let mut truncated = false;
    let mut rows_seen: usize = 0;
    let mut rows_output: usize = 0;
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

        if rows_output >= max_rows_usize {
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
            page_rows = Vec::with_capacity(capacity);
        }
    }
    drop(stream);
    let duration_ms = started.elapsed().as_millis() as u64;

    if rows_output == 0
        && columns.is_none()
        && cellar_core::query::statement_may_return_rows(exec_sql)
    {
        let parameter_types: Vec<_> = bind_values
            .iter()
            .map(|value| crate::bind::type_info(value))
            .collect();
        let described = (&mut *acquired)
            .prepare_with(exec_sql, &parameter_types)
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
        let offset = rows_output - page_rows.len();
        on_page(QueryResultPage {
            columns: columns.clone().unwrap_or_default(),
            rows: page_rows,
            offset: offset as u64,
        })?;
    } else if rows_output == 0 && columns.is_some() {
        on_page(QueryResultPage {
            columns: columns.clone().unwrap_or_default(),
            rows: Vec::new(),
            offset: 0,
        })?;
    }

    // Only surface an affected count for statements without a result set
    // (INSERT/UPDATE/DELETE/DDL without RETURNING). For row-returning
    // statements the tag count is just the row count and the UI would
    // mislabel a SELECT as "N rows affected".
    let rows_affected = if columns.is_none() {
        rows_affected
    } else {
        None
    };

    let columns = columns.unwrap_or_default();
    Ok((columns, QueryResultSummary {
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
        row_count: rows_output as u64,
    }))
}

async fn execute_read_only_query(pool: &PgPool, query: &Query) -> CellarResult<QueryResult> {
    let max_rows = query.max_rows.unwrap_or(DEFAULT_MAX_ROWS);
    let max_rows_usize = max_rows as usize;
    let skip_rows = query.offset.unwrap_or(0) as usize;
    let capacity = max_rows.min(10_000) as usize;
    let (prepared_sql, bind_values) = prepare_query(query)?;
    let exec_sql: &str = prepared_sql.as_ref();
    let started = Instant::now();

    let mut tx = pool.begin().await.map_err(query_sqlx_err)?;
    sqlx::query("SET TRANSACTION READ ONLY")
        .execute(&mut *tx)
        .await
        .map_err(query_sqlx_err)?;

    let mut statement = sqlx::query(exec_sql);
    for &value in &bind_values {
        statement = crate::bind::bind_value(statement, value)?;
    }
    #[allow(deprecated)]
    let mut stream = statement.fetch_many(&mut *tx);
    let mut columns: Option<Vec<ColumnMeta>> = None;
    let mut materialized: Vec<Row> = Vec::with_capacity(capacity);
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
    drop(stream);
    if materialized.is_empty()
        && columns.is_none()
        && cellar_core::query::statement_may_return_rows(exec_sql)
    {
        let described = (&mut *tx)
            .describe(exec_sql)
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
    tx.rollback().await.map_err(query_sqlx_err)?;

    let rows_affected = if columns.is_none() {
        rows_affected
    } else {
        None
    };

    Ok(QueryResult {
        columns: columns.unwrap_or_default(),
        rows: materialized,
        notices: Vec::new(),
        notice_capture: NoticeCapture::unsupported(
            "Postgres server notices are parsed by sqlx, but the current PgPool query path consumes NoticeResponse frames internally and exposes only log/tracing output without SQLSTATE, detail, hint, or query correlation.",
        ),
        rows_affected,
        duration_ms: started.elapsed().as_millis() as u64,
        truncated,
        total_rows: None,
    })
}

fn prepare_query(query: &Query) -> CellarResult<(Cow<'_, str>, Vec<&CellValue>)> {
    if query.params.is_empty() {
        return Ok((Cow::Borrowed(query.sql.as_str()), Vec::new()));
    }

    let prepared = cellar_sql::prepare(&query.sql, Engine::Postgres)
        .map_err(|e| CellarError::query(e.to_string()))?;
    let by_name: HashMap<&str, &CellValue> = query
        .params
        .iter()
        .map(|p| (p.name.as_str(), &p.value))
        .collect();
    let bind_values = cellar_sql::order_values(&prepared.parameters, &by_name)
        .map_err(|e| CellarError::query(e.to_string()))?
        .into_iter()
        .copied()
        .collect();
    Ok((Cow::Owned(prepared.sql), bind_values))
}

/// How the committed row count is checked against the plan's expectation.
#[derive(Clone, Copy)]
enum RowCountCheck {
    /// Grid edits target known-present rows: every statement must affect
    /// exactly one row, so the total must equal the expectation.
    Exact,
    /// CSV imports include no-op rows (an `UPDATE` that matches nothing, an
    /// `ON CONFLICT DO NOTHING` on a duplicate). Those affect zero rows, so the
    /// expectation is only a ceiling — exceeding it means generated SQL touched
    /// more rows than submitted, which still aborts.
    AtMost,
}

pub async fn commit_table_changes(
    conn: &PgConnection,
    request: &TableChangeRequest,
) -> CellarResult<TableCommitResult> {
    commit_plan(conn, request, RowCountCheck::Exact).await
}

/// Commit a CSV import (`Update`/`Upsert` rows) in one transaction. Unlike
/// `commit_table_changes` this tolerates no-op rows, so re-running the same
/// CSV is idempotent rather than a row-count mismatch error.
pub async fn commit_table_import(
    conn: &PgConnection,
    request: &TableChangeRequest,
) -> CellarResult<TableCommitResult> {
    commit_plan(conn, request, RowCountCheck::AtMost).await
}

async fn commit_plan(
    conn: &PgConnection,
    request: &TableChangeRequest,
    check: RowCountCheck,
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
        let statement_rows = result.rows_affected();
        if !statement_row_count_valid(check, statement_rows) {
            tx.rollback().await.map_err(query_sqlx_err)?;
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

    let mismatch = match check {
        RowCountCheck::Exact => rows_affected != plan.preview.expected_rows,
        RowCountCheck::AtMost => rows_affected > plan.preview.expected_rows,
    };
    if mismatch {
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

fn statement_row_count_valid(check: RowCountCheck, rows: u64) -> bool {
    match check {
        RowCountCheck::Exact => rows == 1,
        RowCountCheck::AtMost => rows <= 1,
    }
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
#[path = "query_tests.rs"]
mod tests;
