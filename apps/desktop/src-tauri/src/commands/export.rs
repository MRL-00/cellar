//! Native save-dialog export for query result downloads (SPEC §9.1).
//!
//! The frontend formats CSV/TSV/JSON/SQL in TypeScript; this command only
//! prompts for a destination and writes the bytes the user already approved.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use cellar_core::error::CellarError;
use tokio::fs;
use tokio::io::AsyncWriteExt;
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

    write_atomically(&path, contents.as_bytes()).await?;

    Ok(Some(path.to_string_lossy().into_owned()))
}

/// Write `contents` via a same-directory temp file, sync, then rename over
/// `path` so a failed overwrite cannot leave a truncated destination.
async fn write_atomically(path: &Path, contents: &[u8]) -> Result<(), CellarError> {
    let tmp = temp_sibling(path);
    let io = |e: std::io::Error| {
        CellarError::Io(format!("failed to write {}: {e}", path.display()))
    };

    let write_result = async {
        let mut file = fs::File::create(&tmp).await.map_err(&io)?;
        file.write_all(contents).await.map_err(&io)?;
        file.sync_all().await.map_err(&io)?;
        drop(file);
        fs::rename(&tmp, path).await.map_err(&io)?;
        Ok(())
    }
    .await;

    if write_result.is_err() {
        let _ = fs::remove_file(&tmp).await;
    }
    write_result
}

fn temp_sibling(path: &Path) -> PathBuf {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let stem = path
        .file_name()
        .map(|n| n.to_string_lossy())
        .unwrap_or_else(|| "export".into());
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    parent.join(format!(".{stem}.{}.{nanos}.tmp", std::process::id()))
}
