//! AI provider key storage (SPEC §6.7, §7).
//!
//! Cellar is bring-your-own-key: the provider API key is stored in the OS
//! keychain via [`cellar_secrets`], never written to disk or config, and never
//! logged. The actual provider HTTP call happens in the frontend (SPEC §3), so
//! [`ai_load_key`] hands the key back to the renderer at request time only.
//!
//! Keys are namespaced `ai:<provider>` so they cannot collide with connection
//! credentials, which key on the connection id.

use cellar_core::error::CellarError;

fn entry_name(provider: &str) -> String {
    format!("ai:{provider}")
}

/// Persist the API key for `provider` in the OS keychain, overwriting any
/// existing entry.
#[tauri::command]
#[specta::specta]
pub async fn ai_store_key(provider: String, key: String) -> Result<(), CellarError> {
    cellar_secrets::store(&entry_name(&provider), &key)?;
    Ok(())
}

/// Load the stored API key for `provider`. Returns `None` when nothing is
/// stored, which is distinct from a keychain failure.
#[tauri::command]
#[specta::specta]
pub async fn ai_load_key(provider: String) -> Result<Option<String>, CellarError> {
    Ok(cellar_secrets::load(&entry_name(&provider))?)
}

/// Remove the stored API key for `provider`. A no-op if none exists.
#[tauri::command]
#[specta::specta]
pub async fn ai_delete_key(provider: String) -> Result<(), CellarError> {
    cellar_secrets::delete(&entry_name(&provider))?;
    Ok(())
}

/// Report whether a key is stored for `provider` without returning it — used by
/// settings to show "configured" state.
#[tauri::command]
#[specta::specta]
pub async fn ai_has_key(provider: String) -> Result<bool, CellarError> {
    Ok(cellar_secrets::load(&entry_name(&provider))?.is_some())
}
