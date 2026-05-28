use cellar_core::error::CellarError;
use cellar_core::schema::Database;
use tauri::State;

use crate::state::ConnectionRegistry;

#[tauri::command]
#[specta::specta]
pub async fn introspect(
    registry: State<'_, ConnectionRegistry>,
    connection_id: String,
    refresh: Option<bool>,
) -> Result<Vec<Database>, CellarError> {
    registry
        .introspect(&connection_id, refresh.unwrap_or(false))
        .await
}
