use cellar_core::error::CellarError;
use cellar_diff::{build_postgres_plan, TableChangeRequest, TableCommitPreview, TableCommitResult};
use tauri::State;

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
    connection_id: String,
    request: TableChangeRequest,
) -> Result<TableCommitResult, CellarError> {
    registry.commit_table_changes(&connection_id, request).await
}
