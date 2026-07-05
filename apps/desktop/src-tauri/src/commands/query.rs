use std::time::Instant;

use cellar_core::driver::Engine;
use cellar_core::error::CellarError;
use cellar_core::query::{
    DetectedParameter, PlanMode, Query, QueryParam, QueryPlan, QueryResult, TableBrowseRequest,
};
use tauri::State;

use crate::history::{HistoryStore, NewQueryHistoryRecord};
use crate::state::ConnectionRegistry;

#[tauri::command]
#[specta::specta]
pub async fn run_query(
    registry: State<'_, ConnectionRegistry>,
    history: State<'_, HistoryStore>,
    connection_id: String,
    sql: String,
    max_rows: Option<u32>,
    offset: Option<u32>,
    database: Option<String>,
    tab_id: Option<String>,
    query_id: Option<String>,
    params: Option<Vec<QueryParam>>,
) -> Result<QueryResult, CellarError> {
    let mut query = Query::new(sql);
    if let Some(n) = max_rows {
        query = query.with_max_rows(n);
    }
    if let Some(o) = offset {
        query = query.with_offset(o);
    }
    if let Some(db) = database {
        query = query.with_database(db);
    }
    if let Some(qid) = query_id {
        query = query.with_query_id(qid);
    }
    if let Some(params) = params {
        query = query.with_params(params);
    }
    let history_sql = query.sql.clone();
    let history_database = query.database.clone();
    let context = registry.history_context(&connection_id).await;
    let started = Instant::now();
    let result = registry.run_query(&connection_id, query).await;
    let duration_ms = result
        .as_ref()
        .map(|r| r.duration_ms)
        .unwrap_or_else(|_| started.elapsed().as_millis() as u64) as i64;

    let (success, row_count, truncated, error_summary) = match &result {
        Ok(query_result) => (
            true,
            query_result
                .rows_affected
                .map(|n| n.min(i64::MAX as u64) as i64)
                .or(Some(query_result.rows.len() as i64)),
            query_result.truncated,
            None,
        ),
        Err(err) => (false, None, false, Some(err.to_string())),
    };
    let record = NewQueryHistoryRecord {
        connection_id: connection_id.clone(),
        connection_name: context.name,
        tab_id,
        database: history_database.or(context.database),
        sql: history_sql,
        duration_ms,
        success,
        row_count,
        truncated,
        error_summary,
    };
    // History should be useful, not a new reason for query execution to fail.
    let _ = history.insert(record).await;

    result
}

#[tauri::command]
#[specta::specta]
pub async fn run_read_only_query(
    registry: State<'_, ConnectionRegistry>,
    connection_id: String,
    sql: String,
    max_rows: Option<u32>,
    database: Option<String>,
) -> Result<QueryResult, CellarError> {
    let mut query = Query::new(sql).read_only();
    if let Some(n) = max_rows {
        query = query.with_max_rows(n);
    }
    if let Some(db) = database {
        query = query.with_database(db);
    }
    registry.run_query(&connection_id, query).await
}

/// Cancel a running query previously started through [`run_query`] with a
/// `query_id`. Returns `true` when a running statement was found and
/// signalled, `false` when it had already finished.
#[tauri::command]
#[specta::specta]
pub async fn cancel_query(
    registry: State<'_, ConnectionRegistry>,
    connection_id: String,
    query_id: String,
) -> Result<bool, CellarError> {
    registry.cancel_query(&connection_id, &query_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn explain_query(
    registry: State<'_, ConnectionRegistry>,
    connection_id: String,
    sql: String,
    mode: PlanMode,
    database: Option<String>,
) -> Result<QueryPlan, CellarError> {
    let mut query = Query::new(sql);
    if let Some(db) = database {
        query = query.with_database(db);
    }
    registry.explain_query(&connection_id, query, mode).await
}

/// Detect named (`:name`) and positional (`$N`) parameter placeholders in a
/// statement so the editor can collect values before running it. Pure parsing
/// via `cellar-sql`; no connection is required.
#[tauri::command]
#[specta::specta]
pub async fn detect_query_parameters(
    sql: String,
    engine: Engine,
) -> Result<Vec<DetectedParameter>, CellarError> {
    cellar_sql::prepare(&sql, engine)
        .map(|prepared| prepared.parameters)
        .map_err(|e| CellarError::query(e.to_string()))
}

#[tauri::command]
#[specta::specta]
pub async fn browse_table(
    registry: State<'_, ConnectionRegistry>,
    request: TableBrowseRequest,
) -> Result<QueryResult, CellarError> {
    registry.browse_table(request).await
}
