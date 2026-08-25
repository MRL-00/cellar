//! Native save-dialog export for query result downloads (SPEC §9.1).
//!
//! The frontend formats CSV/TSV/JSON/SQL in TypeScript; this command only
//! prompts for a destination and writes the bytes the user already approved.

use cellar_core::error::CellarError;
use tokio::task::spawn_blocking;

/// Show a platform save dialog prefilled with `default_name`, then write
/// `contents` to the chosen path. Returns `Ok(None)` when the user cancels.
#[tauri::command]
#[specta::specta]
pub async fn save_text_file(
    default_name: String,
    contents: String,
    filter_name: String,
    filter_ext: String,
) -> Result<Option<String>, CellarError> {
    let chosen = spawn_blocking(move || {
        rfd::FileDialog::new()
            .set_file_name(&default_name)
            .add_filter(&filter_name, &[filter_ext.as_str()])
            .save_file()
    })
    .await
    .map_err(|e| CellarError::Internal(format!("save dialog task failed: {e}")))?;

    let Some(path) = chosen else {
        return Ok(None);
    };

    let written_path = path.clone();
    spawn_blocking(move || {
        cellar_runtime::export::write_atomically(&written_path, contents.as_bytes())
    })
    .await
    .map_err(|error| CellarError::Internal(format!("export task failed: {error}")))??;

    Ok(Some(path.to_string_lossy().into_owned()))
}
