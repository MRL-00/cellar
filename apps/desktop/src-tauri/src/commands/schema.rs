use cellar_core::error::CellarError;
use cellar_core::schema::{Database, UsageReference};
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

/// Find every view, function, procedure, trigger, and constraint that
/// references `object_name` (optionally narrowed to `column_name`). Scoped to
/// `schema` within `database` by default; set `all_schemas` to search the whole
/// database. References are confirmed structurally, never by substring match.
#[tauri::command]
#[specta::specta]
#[allow(clippy::too_many_arguments)]
pub async fn find_usages(
    registry: State<'_, ConnectionRegistry>,
    connection_id: String,
    database: Option<String>,
    schema: String,
    object_name: String,
    column_name: Option<String>,
    all_schemas: Option<bool>,
) -> Result<Vec<UsageReference>, CellarError> {
    registry
        .find_usages(
            &connection_id,
            database,
            schema,
            object_name,
            column_name,
            all_schemas.unwrap_or(false),
        )
        .await
}
