//! Local saved-query library shared with the native GPUI client.

pub use cellar_runtime::query_templates::QueryTemplate;

use cellar_core::error::CellarError;

#[tauri::command]
#[specta::specta]
/// List every saved template, sorted by name. Missing directory ⇒ empty list.
pub async fn list_query_templates() -> Result<Vec<QueryTemplate>, CellarError> {
    cellar_runtime::query_templates::list().await
}

#[tauri::command]
#[specta::specta]
/// Save (create or overwrite) a template. Returns the stored template.
pub async fn save_query_template(template: QueryTemplate) -> Result<QueryTemplate, CellarError> {
    cellar_runtime::query_templates::save(template).await
}

#[tauri::command]
#[specta::specta]
/// Delete the template whose name slugifies to `name`'s file.
pub async fn delete_query_template(name: String) -> Result<(), CellarError> {
    cellar_runtime::query_templates::delete(&name).await
}
