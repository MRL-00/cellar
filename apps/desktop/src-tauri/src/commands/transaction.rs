use std::time::Instant;

use cellar_core::error::CellarError;
use cellar_diff::{build_postgres_plan, TableChangeRequest, TableCommitPreview, TableCommitResult};
use tauri::State;

use crate::history::{HistoryStore, NewQueryHistoryRecord};
use crate::state::ConnectionRegistry;

#[tauri::command]
#[specta::specta]
pub fn preview_table_changes(
    request: TableChangeRequest,
) -> Result<TableCommitPreview, CellarError> {
    build_postgres_plan(&request)
        .map(|plan| plan.preview)
        .map_err(|e| CellarError::query(e.to_string()))
}

#[tauri::command]
#[specta::specta]
pub async fn commit_table_changes(
    registry: State<'_, ConnectionRegistry>,
    history: State<'_, HistoryStore>,
    connection_id: String,
    request: TableChangeRequest,
    tab_id: Option<String>,
) -> Result<TableCommitResult, CellarError> {
    let history_database = request.database.clone();
    let history_sql = build_postgres_plan(&request)
        .map(|plan| plan.preview.sql)
        .unwrap_or_else(|err| {
            format!(
                "-- failed to build table change SQL for {}.{}: {}",
                request.schema, request.table, err
            )
        });
    let context = registry.history_context(&connection_id).await;
    let started = Instant::now();
    let result = registry.commit_table_changes(&connection_id, request).await;
    let duration_ms = result
        .as_ref()
        .map(|r| r.duration_ms)
        .unwrap_or_else(|_| started.elapsed().as_millis() as u64) as i64;

    let (success, sql, row_count, error_summary) = match &result {
        Ok(commit_result) => (
            true,
            commit_result.sql.clone(),
            Some(commit_result.rows_affected.min(i64::MAX as u64) as i64),
            None,
        ),
        Err(err) => (false, history_sql, None, Some(err.to_string())),
    };
    let record = NewQueryHistoryRecord {
        connection_id: connection_id.clone(),
        connection_name: context.name,
        tab_id,
        database: history_database.or(context.database),
        sql,
        duration_ms,
        success,
        row_count,
        truncated: false,
        error_summary,
    };
    let _ = history.insert(record).await;

    result
}
