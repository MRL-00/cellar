use std::path::{Path, PathBuf};

use cellar_core::error::{CellarError, CellarResult};
use serde::{Deserialize, Serialize};
use specta::Type;
use tokio::fs;

use crate::cellar_dir;

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
/// A saved (optionally parameterized) query. `name` is the stable identifier;
/// saving a template whose name slugifies to an existing file overwrites it.
pub struct QueryTemplate {
    pub name: String,
    pub description: String,
    pub sql: String,
}

pub async fn list() -> CellarResult<Vec<QueryTemplate>> {
    list_from_dir(&templates_dir()?).await
}

pub async fn save(template: QueryTemplate) -> CellarResult<QueryTemplate> {
    if template.name.trim().is_empty() {
        return Err(CellarError::invalid_config("template name cannot be empty"));
    }
    let dir = templates_dir()?;
    fs::create_dir_all(&dir).await?;
    fs::write(
        dir.join(format!("{}.json", slug(&template.name))),
        serde_json::to_vec_pretty(&template)
            .map_err(|error| CellarError::Internal(error.to_string()))?,
    )
    .await?;
    Ok(template)
}

pub async fn delete(name: &str) -> CellarResult<()> {
    let path = templates_dir()?.join(format!("{}.json", slug(name)));
    match fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn templates_dir() -> CellarResult<PathBuf> {
    cellar_dir()
        .map(|path| path.join("queries"))
        .ok_or_else(|| CellarError::invalid_config("could not resolve home directory"))
}

async fn list_from_dir(dir: &Path) -> CellarResult<Vec<QueryTemplate>> {
    let mut entries = match fs::read_dir(dir).await {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    let mut templates = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
            continue;
        }
        if let Ok(bytes) = fs::read(path).await {
            if let Ok(template) = serde_json::from_slice(&bytes) {
                templates.push(template);
            }
        }
    }
    templates.sort_by_key(|template: &QueryTemplate| template.name.to_ascii_lowercase());
    Ok(templates)
}

fn slug(name: &str) -> String {
    let mut slug = String::with_capacity(name.len());
    let mut dash = false;
    for character in name.chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
            slug.push(character.to_ascii_lowercase());
            dash = false;
        } else if !dash {
            slug.push('-');
            dash = true;
        }
    }
    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        "query".into()
    } else {
        slug.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn templates_round_trip_and_ignore_corrupt_files() {
        let dir = tempfile::tempdir().unwrap();
        let template = QueryTemplate {
            name: "Active users / region".into(),
            description: "Report".into(),
            sql: "select 1".into(),
        };
        fs::create_dir_all(dir.path()).await.unwrap();
        fs::write(
            dir.path().join("active-users-region.json"),
            serde_json::to_vec(&template).unwrap(),
        )
        .await
        .unwrap();
        fs::write(dir.path().join("broken.json"), b"{")
            .await
            .unwrap();
        assert_eq!(list_from_dir(dir.path()).await.unwrap(), vec![template]);
        assert_eq!(slug("Active users / region"), "active-users-region");
    }
}
