use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use cellar_core::driver::{Connection, ConnectionConfig, Driver, DriverInfo, Engine};
use cellar_core::error::{CellarError, CellarResult};
use cellar_core::query::{PlanMode, Query, QueryPlan, QueryResult};
use cellar_core::schema::Database;
use cellar_diff::{TableChangeRequest, TableCommitResult};
use cellar_driver_postgres::PostgresDriver;
use tokio::fs;
use tokio::sync::RwLock;

/// Filename for the on-disk connection list. Lives under `~/.cellar/` —
/// passwords are never written here (see [`cellar_secrets`]).
const STORAGE_FILENAME: &str = "connections.json";

pub struct OpenConnection {
    pub config: ConnectionConfig,
    pub connection: Arc<dyn Connection>,
}

#[derive(Debug, Clone)]
pub struct ConnectionHistoryContext {
    pub name: Option<String>,
    pub database: Option<String>,
}

#[derive(Default)]
struct RegistryInner {
    /// Configs the user has saved. Mirrors `~/.cellar/connections.json`.
    configs: HashMap<String, ConnectionConfig>,
    /// Live pools, keyed by config id.
    open: HashMap<String, OpenConnection>,
    /// Schema-tree cache from the last successful `introspect` call.
    schema_cache: HashMap<String, Vec<Database>>,
}

pub struct ConnectionRegistry {
    inner: RwLock<RegistryInner>,
}

impl ConnectionRegistry {
    /// Load saved connection configs from disk and return a registry seeded
    /// with them. Errors are tolerated — a missing or malformed file gives
    /// you an empty registry rather than a crashed app.
    pub async fn load() -> Self {
        let path = match storage_path() {
            Some(p) => p,
            None => return Self::empty(),
        };
        let raw = match fs::read_to_string(&path).await {
            Ok(s) => s,
            Err(_) => return Self::empty(),
        };
        let configs: Vec<ConnectionConfig> = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(_) => return Self::empty(),
        };
        let map = configs.into_iter().map(|c| (c.id.clone(), c)).collect();
        Self {
            inner: RwLock::new(RegistryInner {
                configs: map,
                open: HashMap::new(),
                schema_cache: HashMap::new(),
            }),
        }
    }

    fn empty() -> Self {
        Self {
            inner: RwLock::new(RegistryInner::default()),
        }
    }

    pub async fn list(&self) -> Vec<ConnectionConfig> {
        let inner = self.inner.read().await;
        let mut out: Vec<_> = inner.configs.values().cloned().collect();
        out.sort_by(|a, b| a.name.cmp(&b.name));
        out
    }

    pub async fn save(&self, config: ConnectionConfig) -> CellarResult<ConnectionConfig> {
        if config.id.is_empty() {
            return Err(CellarError::invalid_config("connection id is empty"));
        }
        {
            let mut inner = self.inner.write().await;
            inner.configs.insert(config.id.clone(), config.clone());
            persist(&inner.configs).await?;
        }
        Ok(config)
    }

    pub async fn delete(&self, id: &str) -> CellarResult<()> {
        let mut inner = self.inner.write().await;
        inner.configs.remove(id);
        if let Some(open) = inner.open.remove(id) {
            // Best-effort close; ignore errors when tearing down.
            let _ = open.connection.close().await;
        }
        inner.schema_cache.remove(id);
        persist(&inner.configs).await?;
        Ok(())
    }

    pub async fn test(
        &self,
        config: &ConnectionConfig,
        password: Option<&str>,
    ) -> CellarResult<DriverInfo> {
        let driver = driver_for(config.engine)?;
        let conn = driver.connect(config, password).await?;
        let info = conn.info().clone();
        let _ = conn.close().await;
        Ok(info)
    }

    pub async fn open_info(&self, id: &str) -> Option<DriverInfo> {
        let inner = self.inner.read().await;
        inner.open.get(id).map(|open| open.connection.info().clone())
    }

    pub async fn connect(&self, id: &str, password: Option<&str>) -> CellarResult<DriverInfo> {
        let config = self.config_for(id).await?;
        {
            let inner = self.inner.read().await;
            if let Some(open) = inner.open.get(id) {
                return Ok(open.connection.info().clone());
            }
        }

        let driver = driver_for(config.engine)?;
        let conn = driver.connect(&config, password).await?;
        let info = conn.info().clone();
        let connection: Arc<dyn Connection> = conn.into();
        let mut inner = self.inner.write().await;
        if let Some(open) = inner.open.get(id) {
            let existing = open.connection.info().clone();
            drop(inner);
            let _ = connection.close().await;
            return Ok(existing);
        }
        inner
            .open
            .insert(id.to_string(), OpenConnection { config, connection });
        Ok(info)
    }

    pub async fn disconnect(&self, id: &str) -> CellarResult<()> {
        let mut inner = self.inner.write().await;
        let removed = inner.open.remove(id);
        inner.schema_cache.remove(id);
        drop(inner);
        if let Some(open) = removed {
            let _ = open.connection.close().await;
        }
        Ok(())
    }

    pub async fn introspect(&self, id: &str, refresh: bool) -> CellarResult<Vec<Database>> {
        if !refresh {
            let inner = self.inner.read().await;
            if let Some(hit) = inner.schema_cache.get(id) {
                return Ok(hit.clone());
            }
        }
        let dbs = {
            let inner = self.inner.read().await;
            let open = inner
                .open
                .get(id)
                .ok_or_else(|| CellarError::NotConnected(format!("no open connection for {id}")))?;
            let engine = open.config.engine;
            let connection = Arc::clone(&open.connection);
            drop(inner);

            let driver = driver_for(engine)?;
            driver.introspect(connection.as_ref()).await?
        };
        let mut w = self.inner.write().await;
        w.schema_cache.insert(id.to_string(), dbs.clone());
        Ok(dbs)
    }

    pub async fn run_query(&self, id: &str, query: Query) -> CellarResult<QueryResult> {
        let (engine, connection) = {
            let inner = self.inner.read().await;
            let open = inner
                .open
                .get(id)
                .ok_or_else(|| CellarError::NotConnected(format!("no open connection for {id}")))?;
            (open.config.engine, Arc::clone(&open.connection))
        };
        let driver = driver_for(engine)?;
        driver.execute_query(connection.as_ref(), &query).await
    }

    pub async fn commit_table_changes(
        &self,
        id: &str,
        request: TableChangeRequest,
    ) -> CellarResult<TableCommitResult> {
        let inner = self.inner.read().await;
        let open = inner
            .open
            .get(id)
            .ok_or_else(|| CellarError::NotConnected(format!("no open connection for {id}")))?;
        match open.config.engine {
            Engine::Postgres => {
                cellar_driver_postgres::commit_table_changes(open.connection.as_ref(), &request)
                    .await
            }
            other => Err(CellarError::invalid_config(format!(
                "engine {} does not support grid commits yet",
                other.as_str()
            ))),
        }
    }

    pub async fn history_context(&self, id: &str) -> ConnectionHistoryContext {
        let inner = self.inner.read().await;
        let config = inner
            .configs
            .get(id)
            .or_else(|| inner.open.get(id).map(|o| &o.config));
        ConnectionHistoryContext {
            name: config.map(|c| c.name.clone()),
            database: config.map(|c| c.database.clone()),
        }
    }

    pub async fn explain_query(
        &self,
        id: &str,
        query: Query,
        mode: PlanMode,
    ) -> CellarResult<QueryPlan> {
        let inner = self.inner.read().await;
        let open = inner
            .open
            .get(id)
            .ok_or_else(|| CellarError::NotConnected(format!("no open connection for {id}")))?;
        if open.config.engine != Engine::Postgres {
            return Err(CellarError::invalid_config(format!(
                "execution plans are not available for {} yet",
                open.config.engine.as_str()
            )));
        }
        let driver = driver_for(open.config.engine)?;
        driver
            .explain_query(open.connection.as_ref(), &query, mode)
            .await
    }

    async fn config_for(&self, id: &str) -> CellarResult<ConnectionConfig> {
        let inner = self.inner.read().await;
        inner
            .configs
            .get(id)
            .cloned()
            .ok_or_else(|| CellarError::invalid_config(format!("unknown connection {id}")))
    }
}

fn driver_for(engine: Engine) -> CellarResult<Box<dyn Driver>> {
    match engine {
        Engine::Postgres => Ok(Box::new(PostgresDriver::new())),
        other => Err(CellarError::invalid_config(format!(
            "engine {} is not supported in this build",
            other.as_str()
        ))),
    }
}

fn storage_dir() -> Option<PathBuf> {
    let mut p = dirs::home_dir()?;
    p.push(".cellar");
    Some(p)
}

fn storage_path() -> Option<PathBuf> {
    let mut p = storage_dir()?;
    p.push(STORAGE_FILENAME);
    Some(p)
}

async fn persist(configs: &HashMap<String, ConnectionConfig>) -> CellarResult<()> {
    let dir = storage_dir()
        .ok_or_else(|| CellarError::invalid_config("could not resolve home directory"))?;
    fs::create_dir_all(&dir).await?;
    let mut path = dir.clone();
    path.push(STORAGE_FILENAME);
    let mut list: Vec<_> = configs.values().cloned().collect();
    list.sort_by(|a, b| a.name.cmp(&b.name));
    let json = serde_json::to_string_pretty(&list)?;
    fs::write(&path, json).await?;
    Ok(())
}
