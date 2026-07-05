//! Schema comparison, migration-script assembly, and snapshot commands.
//!
//! Two schemas (each a live connection or a saved snapshot) are compared into
//! a typed diff plus an ordered migration. The migration script is assembled
//! from the user's selected statements and only applied after explicit review
//! (see SPEC §6.5's Review & Commit pattern). Snapshots live under
//! `~/.cellar/snapshots/` for offline comparison.

use std::path::PathBuf;
use std::time::Instant;

use cellar_core::driver::Engine;
use cellar_core::error::CellarError;
use cellar_core::schema::Schema;
use cellar_schema_diff::{
    assemble_script, compare as compare_schemas_impl, Dialect, MigrationStatement,
    SchemaComparison, SchemaSnapshot, SchemaSnapshotMeta,
};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::State;
use tokio::fs;

use crate::history::{HistoryStore, NewQueryHistoryRecord};
use crate::state::ConnectionRegistry;

const SNAPSHOTS_DIRNAME: &str = "snapshots";

/// One side of a comparison. A schema either comes from a live connection
/// (introspected fresh) or from a saved snapshot on disk.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum SchemaSource {
    Live {
        connection_id: String,
        database: String,
        schema: String,
        label: Option<String>,
    },
    Snapshot {
        id: String,
        schema: String,
        label: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct MigrationApplyResult {
    pub duration_ms: u64,
}

/// Compare `source` against `target`, returning the render-ready diff and the
/// migration statements that transform source into target.
#[tauri::command]
#[specta::specta]
pub async fn compare_schemas(
    registry: State<'_, ConnectionRegistry>,
    source: SchemaSource,
    target: SchemaSource,
) -> Result<SchemaComparison, CellarError> {
    let (source_schema, source_label, dialect) = resolve_source(&registry, &source).await?;
    let (target_schema, target_label, _) = resolve_source(&registry, &target).await?;

    // The migration is applied to the source to make it match the target, so
    // DDL is qualified with the source's schema namespace.
    let namespace = source_schema.name.clone();
    Ok(compare_schemas_impl(
        &source_schema,
        &target_schema,
        source_label,
        target_label,
        &namespace,
        dialect,
    ))
}

/// Assemble the user's selected statements into a single runnable script.
/// Kept in Rust so transaction wrapping stays dialect-aware and the frontend
/// never hand-builds executable SQL. The `dialect` comes from the comparison
/// (the source engine) so the transaction wrap matches that engine's DDL
/// semantics rather than always assuming Postgres.
#[tauri::command]
#[specta::specta]
pub fn build_migration_script(
    statements: Vec<MigrationStatement>,
    dialect: Dialect,
    wrap_in_transaction: bool,
) -> String {
    assemble_script(&statements, dialect, wrap_in_transaction)
}

/// Apply a reviewed (and possibly hand-edited) migration script against
/// `database` on the open connection. Logged to query history like any other
/// executed statement.
#[tauri::command]
#[specta::specta]
pub async fn apply_migration(
    registry: State<'_, ConnectionRegistry>,
    history: State<'_, HistoryStore>,
    connection_id: String,
    database: String,
    sql: String,
    tab_id: Option<String>,
) -> Result<MigrationApplyResult, CellarError> {
    let context = registry.history_context(&connection_id).await;
    let started = Instant::now();
    let result = registry
        .apply_migration(&connection_id, &database, &sql)
        .await;
    let duration_ms = result
        .as_ref()
        .copied()
        .unwrap_or_else(|_| started.elapsed().as_millis() as u64) as i64;

    let record = NewQueryHistoryRecord {
        connection_id: connection_id.clone(),
        connection_name: context.name,
        tab_id,
        database: Some(database),
        sql,
        duration_ms,
        success: result.is_ok(),
        row_count: None,
        truncated: false,
        error_summary: result.as_ref().err().map(|e| e.to_string()),
    };
    let _ = history.insert(record).await;

    result.map(|duration_ms| MigrationApplyResult { duration_ms })
}

/// Capture a live database's schema tree to `~/.cellar/snapshots/`.
#[tauri::command]
#[specta::specta]
pub async fn save_schema_snapshot(
    registry: State<'_, ConnectionRegistry>,
    connection_id: String,
    database: String,
) -> Result<SchemaSnapshotMeta, CellarError> {
    let db = registry.database_for(&connection_id, &database).await?;
    let engine = registry
        .engine_for(&connection_id)
        .await
        .unwrap_or(Engine::Postgres)
        .as_str()
        .to_string();
    let context = registry.history_context(&connection_id).await;
    let connection_name = context.name.unwrap_or_else(|| connection_id.clone());
    let created_at_ms = crate::history::now_ms();
    let id = format!(
        "{}-{}",
        sanitize_id(&format!("{connection_name}-{database}")),
        created_at_ms
    );
    let schemas = db.schemas.iter().map(|s| s.name.clone()).collect();
    let table_count = db.schemas.iter().map(|s| s.tables.len()).sum::<usize>() as u32;

    let meta = SchemaSnapshotMeta {
        id: id.clone(),
        label: format!("{connection_name} · {database}"),
        engine,
        connection_id,
        connection_name,
        database,
        schemas,
        table_count,
        created_at_ms,
    };
    let snapshot = SchemaSnapshot {
        meta: meta.clone(),
        database: db,
    };
    write_snapshot(&id, &snapshot).await?;
    Ok(meta)
}

/// List saved snapshots, newest first.
#[tauri::command]
#[specta::specta]
pub async fn list_schema_snapshots() -> Result<Vec<SchemaSnapshotMeta>, CellarError> {
    let Some(dir) = snapshots_dir() else {
        return Ok(Vec::new());
    };
    let mut entries = match fs::read_dir(&dir).await {
        Ok(entries) => entries,
        // A missing directory just means no snapshots have been saved yet.
        Err(_) => return Ok(Vec::new()),
    };
    let mut metas = Vec::new();
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if let Ok(raw) = fs::read_to_string(&path).await {
            if let Ok(snapshot) = serde_json::from_str::<SchemaSnapshot>(&raw) {
                metas.push(snapshot.meta);
            }
        }
    }
    metas.sort_by_key(|m| std::cmp::Reverse(m.created_at_ms));
    Ok(metas)
}

#[tauri::command]
#[specta::specta]
pub async fn delete_schema_snapshot(id: String) -> Result<(), CellarError> {
    let path = snapshot_path(&id)?;
    match fs::remove_file(&path).await {
        Ok(()) => Ok(()),
        // Treat an already-absent file as success — the desired end state.
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(CellarError::Io(err.to_string())),
    }
}

async fn resolve_source(
    registry: &ConnectionRegistry,
    source: &SchemaSource,
) -> Result<(Schema, String, Dialect), CellarError> {
    match source {
        SchemaSource::Live {
            connection_id,
            database,
            schema,
            label,
        } => {
            let resolved = registry.schema_for(connection_id, database, schema).await?;
            let dialect = dialect_for(registry.engine_for(connection_id).await);
            let label = label
                .clone()
                .unwrap_or_else(|| format!("{database} / {schema}"));
            Ok((resolved, label, dialect))
        }
        SchemaSource::Snapshot { id, schema, label } => {
            let snapshot = read_snapshot(id).await?;
            let resolved = snapshot
                .database
                .schemas
                .into_iter()
                .find(|s| &s.name == schema)
                .ok_or_else(|| {
                    CellarError::invalid_config(format!(
                        "snapshot {id} does not contain schema {schema}"
                    ))
                })?;
            let dialect = dialect_for_str(&snapshot.meta.engine);
            let label = label
                .clone()
                .unwrap_or_else(|| format!("{} / {schema}", snapshot.meta.label));
            Ok((resolved, label, dialect))
        }
    }
}

fn dialect_for(engine: Option<Engine>) -> Dialect {
    match engine.map(|e| e.family()) {
        Some(Engine::MySql) => Dialect::MySql,
        Some(Engine::Sqlite) => Dialect::Sqlite,
        Some(Engine::Mssql) => Dialect::Mssql,
        // Postgres, Firestore (no DDL), or unknown all fall back to Postgres
        // quoting — the only fully implemented DDL dialect today.
        _ => Dialect::Postgres,
    }
}

fn dialect_for_str(engine: &str) -> Dialect {
    match engine {
        "mysql" => Dialect::MySql,
        "sqlite" => Dialect::Sqlite,
        "mssql" | "azure" => Dialect::Mssql,
        _ => Dialect::Postgres,
    }
}

/// Reduce an arbitrary label to a filesystem-safe slug.
fn sanitize_id(value: &str) -> String {
    let slug: String = value
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let trimmed = slug.trim_matches('-');
    if trimmed.is_empty() {
        "snapshot".to_string()
    } else {
        trimmed.to_lowercase()
    }
}

/// Reject ids that could escape the snapshots directory. Snapshot ids we mint
/// only contain `[a-z0-9-]`; anything else is treated as untrusted input.
fn validate_id(id: &str) -> Result<(), CellarError> {
    if id.is_empty()
        || !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(CellarError::invalid_config(format!(
            "invalid snapshot id: {id}"
        )));
    }
    Ok(())
}

fn snapshots_dir() -> Option<PathBuf> {
    let mut p = crate::state::cellar_dir()?;
    p.push(SNAPSHOTS_DIRNAME);
    Some(p)
}

fn snapshot_path(id: &str) -> Result<PathBuf, CellarError> {
    validate_id(id)?;
    let mut dir = snapshots_dir()
        .ok_or_else(|| CellarError::invalid_config("could not resolve home directory"))?;
    dir.push(format!("{id}.json"));
    Ok(dir)
}

async fn write_snapshot(id: &str, snapshot: &SchemaSnapshot) -> Result<(), CellarError> {
    let path = snapshot_path(id)?;
    let dir = path
        .parent()
        .ok_or_else(|| CellarError::invalid_config("could not resolve snapshots directory"))?;
    fs::create_dir_all(dir).await?;
    let json = serde_json::to_string_pretty(snapshot)?;
    // Atomic write: temp file then rename, matching connections.json handling.
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, json).await?;
    fs::rename(&tmp, &path).await?;
    Ok(())
}

async fn read_snapshot(id: &str) -> Result<SchemaSnapshot, CellarError> {
    let path = snapshot_path(id)?;
    let raw = fs::read_to_string(&path)
        .await
        .map_err(|err| CellarError::Io(format!("could not read snapshot {id}: {err}")))?;
    let snapshot = serde_json::from_str(&raw)?;
    Ok(snapshot)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_path_traversal_ids() {
        assert!(validate_id("../etc/passwd").is_err());
        assert!(validate_id("a/b").is_err());
        assert!(validate_id("ok-id_123").is_ok());
    }

    #[test]
    fn sanitizes_labels_into_slugs() {
        assert_eq!(sanitize_id("Prod DB / shop_eu"), "prod-db---shop-eu");
        assert_eq!(sanitize_id("***"), "snapshot");
    }
}
