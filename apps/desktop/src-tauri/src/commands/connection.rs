use cellar_core::driver::{ConnectionConfig, DriverInfo};
use cellar_core::error::CellarError;
use tauri::State;

use crate::datagrip::{self, DatagripImport};
use crate::state::ConnectionRegistry;

#[tauri::command]
#[specta::specta]
pub async fn list_connections(
    registry: State<'_, ConnectionRegistry>,
) -> Result<Vec<ConnectionConfig>, CellarError> {
    Ok(registry.list().await)
}

#[tauri::command]
#[specta::specta]
pub async fn save_connection(
    registry: State<'_, ConnectionRegistry>,
    config: ConnectionConfig,
    password: Option<String>,
) -> Result<ConnectionConfig, CellarError> {
    registry.save_with_secret(config, password.as_deref()).await
}

#[tauri::command]
#[specta::specta]
pub async fn delete_connection(
    registry: State<'_, ConnectionRegistry>,
    id: String,
) -> Result<(), CellarError> {
    registry.delete_with_secret(&id).await
}

#[tauri::command]
#[specta::specta]
pub async fn test_connection(
    registry: State<'_, ConnectionRegistry>,
    config: ConnectionConfig,
    password: Option<String>,
) -> Result<DriverInfo, CellarError> {
    registry.test_with_secret(&config, password).await
}

#[tauri::command]
#[specta::specta]
pub async fn connect(
    registry: State<'_, ConnectionRegistry>,
    id: String,
) -> Result<DriverInfo, CellarError> {
    registry.connect_saved(&id).await
}

#[tauri::command]
#[specta::specta]
pub async fn reconnect(
    registry: State<'_, ConnectionRegistry>,
    id: String,
) -> Result<DriverInfo, CellarError> {
    registry.reconnect_saved(&id).await
}

/// Scan the local machine for DataGrip data sources and return the connections
/// we can map. Read-only — nothing is saved; the frontend lets the user pick
/// which to import and supply passwords (DataGrip never exposes those).
#[tauri::command]
#[specta::specta]
pub async fn import_datagrip() -> Result<DatagripImport, CellarError> {
    tauri::async_runtime::spawn_blocking(datagrip::scan)
        .await
        .map_err(|e| CellarError::invalid_config(format!("DataGrip scan failed: {e}")))
}

#[tauri::command]
#[specta::specta]
pub async fn disconnect(
    registry: State<'_, ConnectionRegistry>,
    id: String,
) -> Result<(), CellarError> {
    registry.disconnect(&id).await
}
