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
    if let Some(p) = password.as_deref() {
        cellar_secrets::store(&config.id, p)?;
    }
    registry.save(config).await
}

#[tauri::command]
#[specta::specta]
pub async fn delete_connection(
    registry: State<'_, ConnectionRegistry>,
    id: String,
) -> Result<(), CellarError> {
    // Best-effort: a missing entry in the keychain is not an error here.
    let _ = cellar_secrets::delete(&id);
    registry.delete(&id).await
}

#[tauri::command]
#[specta::specta]
pub async fn test_connection(
    registry: State<'_, ConnectionRegistry>,
    config: ConnectionConfig,
    password: Option<String>,
) -> Result<DriverInfo, CellarError> {
    let pw_owned: Option<String> = match password {
        Some(p) => Some(p),
        // Fall back to the stored secret so the user can retest a saved
        // connection without retyping the password.
        None => cellar_secrets::load(&config.id).ok().flatten(),
    };
    registry.test(&config, pw_owned.as_deref()).await
}

#[tauri::command]
#[specta::specta]
pub async fn connect(
    registry: State<'_, ConnectionRegistry>,
    id: String,
) -> Result<DriverInfo, CellarError> {
    let password = cellar_secrets::load(&id).ok().flatten();
    registry.connect(&id, password.as_deref()).await
}

#[tauri::command]
#[specta::specta]
pub async fn reconnect(
    registry: State<'_, ConnectionRegistry>,
    id: String,
) -> Result<DriverInfo, CellarError> {
    let password = cellar_secrets::load(&id).ok().flatten();
    registry.reconnect(&id, password.as_deref()).await
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
