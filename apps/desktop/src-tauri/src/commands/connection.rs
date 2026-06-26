use cellar_core::driver::{ConnectionConfig, DriverInfo};
use cellar_core::error::CellarError;
use tauri::State;

use crate::state::ConnectionRegistry;

/// The stored secret for a connection, or `None` when nothing is saved or the
/// keychain read fails — callers fall back to a passwordless attempt either way.
fn stored_password(id: &str) -> Option<String> {
    cellar_secrets::load(id).ok().flatten()
}

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
    // Fall back to the stored secret so the user can retest a saved
    // connection without retyping the password.
    let pw_owned = password.or_else(|| stored_password(&config.id));
    registry.test(&config, pw_owned.as_deref()).await
}

#[tauri::command]
#[specta::specta]
pub async fn connect(
    registry: State<'_, ConnectionRegistry>,
    id: String,
) -> Result<DriverInfo, CellarError> {
    let password = stored_password(&id);
    registry.connect(&id, password.as_deref()).await
}

#[tauri::command]
#[specta::specta]
pub async fn reconnect(
    registry: State<'_, ConnectionRegistry>,
    id: String,
) -> Result<DriverInfo, CellarError> {
    let password = stored_password(&id);
    registry.reconnect(&id, password.as_deref()).await
}

#[tauri::command]
#[specta::specta]
pub async fn disconnect(
    registry: State<'_, ConnectionRegistry>,
    id: String,
) -> Result<(), CellarError> {
    registry.disconnect(&id).await
}
