//! AI provider credentials and OpenAI execution (SPEC §6.7, §7).
//!
//! Cellar is bring-your-own-key: the provider API key is stored in the OS
//! keychain via [`cellar_secrets`], never written to disk or config, and never
//! logged. Legacy frontend providers can load their key through [`ai_load_key`].
//! OpenAI is intentionally different: both Responses API calls and ChatGPT
//! subscription access run in Rust, so OpenAI credentials never enter the
//! renderer process.
//!
//! Keys are namespaced `ai:<provider>` so they cannot collide with connection
//! credentials, which key on the connection id.

use cellar_core::error::CellarError;
use tauri::State;

use crate::openai::{
    OpenAiAuthMode, OpenAiGenerateRequest, OpenAiGenerateResult, OpenAiLoginMethod,
    OpenAiLoginStart, OpenAiModel, OpenAiOAuthStatus, OpenAiService,
};

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
    if provider == "openai" {
        return Err(CellarError::InvalidConfig(
            "OpenAI credentials are backend-only and cannot be loaded by the renderer.".into(),
        ));
    }
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

#[tauri::command]
#[specta::specta]
pub async fn ai_openai_oauth_status(
    service: State<'_, OpenAiService>,
) -> Result<OpenAiOAuthStatus, CellarError> {
    service.oauth_status().await
}

#[tauri::command]
#[specta::specta]
pub async fn ai_openai_start_login(
    service: State<'_, OpenAiService>,
    method: OpenAiLoginMethod,
) -> Result<OpenAiLoginStart, CellarError> {
    service.start_login(method).await
}

#[tauri::command]
#[specta::specta]
pub async fn ai_openai_cancel_login(
    service: State<'_, OpenAiService>,
    login_id: String,
) -> Result<(), CellarError> {
    service.cancel_login(&login_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn ai_openai_logout(service: State<'_, OpenAiService>) -> Result<(), CellarError> {
    service.logout().await
}

#[tauri::command]
#[specta::specta]
pub async fn ai_openai_list_models(
    service: State<'_, OpenAiService>,
    auth_mode: OpenAiAuthMode,
) -> Result<Vec<OpenAiModel>, CellarError> {
    service.list_models(auth_mode).await
}

#[tauri::command]
#[specta::specta]
pub async fn ai_openai_generate(
    service: State<'_, OpenAiService>,
    auth_mode: OpenAiAuthMode,
    request: OpenAiGenerateRequest,
) -> Result<OpenAiGenerateResult, CellarError> {
    service.generate(auth_mode, request).await
}
