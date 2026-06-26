//! Local saved-query library (SPEC §12, post-1.0 "parameterized queries, query
//! templates"). Templates are plain JSON files under `~/.cellar/queries/`, one
//! per template. This is a local-only feature — there is no server-side
//! template storage, by design.

use std::path::{Path, PathBuf};

use cellar_core::error::CellarError;
use serde::{Deserialize, Serialize};
use specta::Type;
use tokio::fs;

/// A saved (optionally parameterized) query. `name` is the stable identifier;
/// saving a template whose name slugifies to an existing file overwrites it.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct QueryTemplate {
    pub name: String,
    pub description: String,
    pub sql: String,
}

/// Directory holding saved query templates: `~/.cellar/queries/`.
fn templates_dir() -> Option<PathBuf> {
    let mut p = dirs::home_dir()?;
    p.push(".cellar");
    p.push("queries");
    Some(p)
}

fn dir_or_err() -> Result<PathBuf, CellarError> {
    templates_dir().ok_or_else(|| CellarError::invalid_config("could not resolve home directory"))
}

/// Turn a template name into a safe, stable filename stem. Keeps alphanumerics,
/// `-`, and `_`; collapses everything else to `-`. Empty input falls back to
/// `query` so a file is always produced.
fn slug(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut last_dash = false;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            out.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "query".to_string()
    } else {
        trimmed
    }
}

/// List every saved template, sorted by name. Missing directory ⇒ empty list.
#[tauri::command]
#[specta::specta]
pub async fn list_query_templates() -> Result<Vec<QueryTemplate>, CellarError> {
    let dir = dir_or_err()?;
    list_from_dir(&dir).await
}

/// Save (create or overwrite) a template. Returns the stored template.
#[tauri::command]
#[specta::specta]
pub async fn save_query_template(template: QueryTemplate) -> Result<QueryTemplate, CellarError> {
    let dir = dir_or_err()?;
    save_to_dir(&dir, &template).await?;
    Ok(template)
}

/// Delete the template whose name slugifies to `name`'s file.
#[tauri::command]
#[specta::specta]
pub async fn delete_query_template(name: String) -> Result<(), CellarError> {
    let dir = dir_or_err()?;
    delete_from_dir(&dir, &name).await
}

async fn list_from_dir(dir: &Path) -> Result<Vec<QueryTemplate>, CellarError> {
    let mut entries = match fs::read_dir(dir).await {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };

    let mut templates = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        // Skip unreadable/corrupt files rather than failing the whole list.
        let Ok(bytes) = fs::read(&path).await else {
            continue;
        };
        if let Ok(template) = serde_json::from_slice::<QueryTemplate>(&bytes) {
            templates.push(template);
        }
    }
    templates.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(templates)
}

async fn save_to_dir(dir: &Path, template: &QueryTemplate) -> Result<(), CellarError> {
    if template.name.trim().is_empty() {
        return Err(CellarError::invalid_config("template name cannot be empty"));
    }
    fs::create_dir_all(dir).await?;
    let path = dir.join(format!("{}.json", slug(&template.name)));
    let json =
        serde_json::to_vec_pretty(template).map_err(|e| CellarError::Internal(e.to_string()))?;
    fs::write(&path, json).await?;
    Ok(())
}

async fn delete_from_dir(dir: &Path, name: &str) -> Result<(), CellarError> {
    let path = dir.join(format!("{}.json", slug(name)));
    match fs::remove_file(&path).await {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_sanitizes_names() {
        assert_eq!(slug("Active users by region"), "active-users-by-region");
        assert_eq!(slug("orders/2026"), "orders-2026");
        assert_eq!(slug("  "), "query");
        assert_eq!(slug("snake_case-OK"), "snake_case-ok");
    }

    #[tokio::test]
    async fn saves_lists_and_deletes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path();

        assert!(list_from_dir(path).await.unwrap().is_empty());

        let t1 = QueryTemplate {
            name: "Recent orders".into(),
            description: "Orders since a date".into(),
            sql: "SELECT * FROM orders WHERE created_at > :since".into(),
        };
        let t2 = QueryTemplate {
            name: "By id".into(),
            description: String::new(),
            sql: "SELECT * FROM users WHERE id = :id".into(),
        };
        save_to_dir(path, &t1).await.unwrap();
        save_to_dir(path, &t2).await.unwrap();

        let listed = list_from_dir(path).await.unwrap();
        assert_eq!(listed.len(), 2);
        // Sorted by name: "By id" before "Recent orders".
        assert_eq!(listed[0].name, "By id");
        assert_eq!(listed[1], t1);

        // Saving the same name overwrites rather than duplicating.
        let t1b = QueryTemplate {
            sql: "SELECT 1".into(),
            ..t1.clone()
        };
        save_to_dir(path, &t1b).await.unwrap();
        let listed = list_from_dir(path).await.unwrap();
        assert_eq!(listed.len(), 2);

        delete_from_dir(path, "Recent orders").await.unwrap();
        let listed = list_from_dir(path).await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "By id");

        // Deleting a missing template is a no-op.
        delete_from_dir(path, "nope").await.unwrap();
    }
}
