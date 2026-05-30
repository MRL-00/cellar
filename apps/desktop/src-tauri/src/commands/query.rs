use std::time::Instant;

use cellar_core::error::CellarError;
use cellar_core::query::{PlanMode, Query, QueryPlan, QueryResult};
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
    database: Option<String>,
    tab_id: Option<String>,
) -> Result<QueryResult, CellarError> {
    let mut query = Query::new(sql);
    if let Some(n) = max_rows {
        query = query.with_max_rows(n);
    }
    if let Some(db) = database {
        query = query.with_database(db);
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

    let record = match &result {
        Ok(query_result) => NewQueryHistoryRecord {
            connection_id: connection_id.clone(),
            connection_name: context.name,
            tab_id,
            database: history_database.or(context.database),
            sql: history_sql,
            duration_ms,
            success: true,
            row_count: query_result
                .rows_affected
                .map(|n| n.min(i64::MAX as u64) as i64)
                .or_else(|| Some(query_result.rows.len() as i64)),
            truncated: query_result.truncated,
            error_summary: None,
        },
        Err(err) => NewQueryHistoryRecord {
            connection_id: connection_id.clone(),
            connection_name: context.name,
            tab_id,
            database: history_database.or(context.database),
            sql: history_sql,
            duration_ms,
            success: false,
            row_count: None,
            truncated: false,
            error_summary: Some(err.to_string()),
        },
    };
    // History should be useful, not a new reason for query execution to fail.
    let _ = history.insert(record).await;

    result
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
