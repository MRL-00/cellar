//! Free-standing helpers for [`super::ConnectionRegistry`]: driver lookup,
//! table resolution, connection-error classification, and on-disk persistence
//! of `~/.cellar/connections.json`. Split out of `state/mod.rs` purely to keep
//! that file under the repo's line-count limit — no behavior changes.

use std::collections::HashMap;
use std::path::PathBuf;

use cellar_core::driver::{ConnectionConfig, Driver, Engine};
use cellar_core::error::{CellarError, CellarResult};
use cellar_core::schema::{Database, Table};
use cellar_driver_convex::ConvexDriver;
use cellar_driver_firestore::FirestoreDriver;
use cellar_driver_mysql::MySqlDriver;
use cellar_driver_postgres::PostgresDriver;
use cellar_driver_sqlite::SqliteDriver;
use cellar_driver_sqlserver::SqlServerDriver;
use tokio::fs;

use super::STORAGE_FILENAME;

pub(super) fn should_evict_connection(err: &CellarError) -> bool {
    matches!(
        err,
        CellarError::Connection(_) | CellarError::Tls(_) | CellarError::NotConnected(_)
    )
}

pub(super) fn reconnectable_error(err: CellarError) -> CellarError {
    CellarError::NotConnected(format!(
        "The database connection was lost and the stale pool was closed. Reconnect and retry the action. Last error: {err}"
    ))
}

pub(super) fn find_table(
    dbs: &[Database],
    database: &str,
    schema: &str,
    table: &str,
) -> CellarResult<Table> {
    let target_schema = dbs
        .iter()
        .find(|db| db.name == database)
        .and_then(|db| db.schemas.iter().find(|s| s.name == schema));

    // Tables first, then fall back to views: a view is selectable just like a
    // table, so browsing one means synthesizing a Table from it (no primary
    // key / foreign keys / indexes, so the browse query simply skips ORDER BY).
    target_schema
        .and_then(|s| s.tables.iter().find(|t| t.name == table).cloned())
        .or_else(|| {
            target_schema
                .and_then(|s| s.views.iter().find(|v| v.name == table))
                .map(|v| Table {
                    name: v.name.clone(),
                    schema: v.schema.clone(),
                    row_count: None,
                    columns: v.columns.clone(),
                    primary_key: Vec::new(),
                    foreign_keys: Vec::new(),
                    indexes: Vec::new(),
                })
        })
        .ok_or_else(|| {
            CellarError::invalid_config(format!(
                "relation {database}.{schema}.{table} was not found in schema metadata"
            ))
        })
}

pub(super) fn driver_for(engine: Engine) -> CellarResult<Box<dyn Driver>> {
    match engine {
        Engine::Convex => Ok(Box::new(ConvexDriver::default())),
        Engine::Firestore => Ok(Box::new(FirestoreDriver::default())),
        // Supabase and Neon speak the Postgres wire protocol; PlanetScale
        // speaks MySQL's. They share the base engine's driver, which reports
        // the connection's own engine via `ConnectionConfig`.
        Engine::MySql | Engine::PlanetScale => Ok(Box::new(MySqlDriver::default())),
        Engine::Postgres | Engine::Supabase | Engine::Neon => {
            Ok(Box::new(PostgresDriver::default()))
        }
        Engine::Sqlite => Ok(Box::new(SqliteDriver::default())),
        Engine::Mssql => Ok(Box::new(SqlServerDriver::new())),
        Engine::Azure => Ok(Box::new(SqlServerDriver::azure())),
    }
}

/// The `~/.cellar/` application data directory. Shared by the connection
/// store, query history, snapshots, and templates.
pub(crate) fn cellar_dir() -> Option<PathBuf> {
    let mut p = dirs::home_dir()?;
    p.push(".cellar");
    Some(p)
}

pub(super) fn storage_path() -> Option<PathBuf> {
    let mut p = cellar_dir()?;
    p.push(STORAGE_FILENAME);
    Some(p)
}

pub(super) async fn persist(configs: &HashMap<String, ConnectionConfig>) -> CellarResult<()> {
    let dir = cellar_dir()
        .ok_or_else(|| CellarError::invalid_config("could not resolve home directory"))?;
    persist_to_dir(configs, &dir).await
}

/// Persist `configs` into `dir/connections.json`. Extracted from [`persist`] so
/// that tests can supply a temporary directory without touching `~/.cellar/`.
///
/// NOTE: The temp file path (`connections.json.tmp`) is fixed within the
/// directory. Within a single process this is safe because the write-lock is
/// held across the IO (a pre-existing lock-across-IO pattern), preventing
/// concurrent callers from interleaving. Running two app instances against the
/// same `~/.cellar/` directory is an unsupported scenario; using a process- or
/// call-unique temp name (e.g. via the `tempfile` crate) would eliminate this
/// latent risk but is left as a follow-up.
pub(super) async fn persist_to_dir(
    configs: &HashMap<String, ConnectionConfig>,
    dir: &std::path::Path,
) -> CellarResult<()> {
    fs::create_dir_all(dir).await?;
    let path = dir.join(STORAGE_FILENAME);
    let mut list: Vec<_> = configs.values().cloned().collect();
    list.sort_by(|a, b| a.name.cmp(&b.name));
    let json = serde_json::to_string_pretty(&list)?;
    // Write to a temp file then atomically rename so a crash mid-write never
    // leaves connections.json in a partially-written state.
    let tmp_path = path.with_extension("json.tmp");
    fs::write(&tmp_path, json).await?;
    fs::rename(&tmp_path, &path).await?;
    Ok(())
}
