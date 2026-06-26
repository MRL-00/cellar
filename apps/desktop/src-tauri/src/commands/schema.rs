use cellar_core::er::{build_er_graph, ErGraph};
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

/// Build the foreign-key graph for the ER diagram view. Reuses the cached
/// introspection tree (so it never re-hits the server when the schema is warm)
/// and derives the graph in [`cellar_core::er`]. `schemas` scopes the graph to
/// a subset of schemas; `None` includes them all.
#[tauri::command]
#[specta::specta]
pub async fn er_graph(
    registry: State<'_, ConnectionRegistry>,
    connection_id: String,
    database: String,
    schemas: Option<Vec<String>>,
) -> Result<ErGraph, CellarError> {
    let databases = registry.introspect(&connection_id, false).await?;
    build_er_graph(&databases, &database, schemas.as_deref()).ok_or_else(|| {
        CellarError::invalid_config(format!(
            "database {database} was not found in schema metadata"
        ))
    })
}
