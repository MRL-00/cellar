use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use cellar_core::driver::{Connection, ConnectionConfig, Driver, DriverInfo, Engine};
use cellar_core::error::{CellarError, CellarResult};
use cellar_core::query::{PlanMode, Query, QueryPlan, QueryResult, TableBrowseRequest};
use cellar_core::schema::{Database, Table};
use cellar_diff::{TableChangeRequest, TableCommitResult};
use cellar_driver_firestore::FirestoreDriver;
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
        inner
            .open
            .get(id)
            .map(|open| open.connection.info().clone())
    }

    pub async fn connect(&self, id: &str, password: Option<&str>) -> CellarResult<DriverInfo> {
        let config = self.config_for(id).await?;
        let existing = {
            let inner = self.inner.read().await;
            inner
                .open
                .get(id)
                .map(|open| (open.connection.info().clone(), Arc::clone(&open.connection)))
        };
        if let Some((info, connection)) = existing {
            if connection.ping().await.is_ok() {
                return Ok(info);
            }
            if let Some(open) = self.remove_open_if_same(id, &connection).await {
                let _ = open.connection.close().await;
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

    pub async fn reconnect(&self, id: &str, password: Option<&str>) -> CellarResult<DriverInfo> {
        if let Some(open) = self.remove_open(id).await {
            let _ = open.connection.close().await;
        }
        self.connect(id, password).await
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
            match driver.introspect(connection.as_ref()).await {
                Ok(dbs) => dbs,
                Err(err) => return Err(self.handle_operation_error(id, err).await),
            }
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
        match driver.execute_query(connection.as_ref(), &query).await {
            Ok(result) => Ok(result),
            Err(err) => Err(self.handle_operation_error(id, err).await),
        }
    }

    pub async fn browse_table(&self, request: TableBrowseRequest) -> CellarResult<QueryResult> {
        let target_database = self.target_database_for(&request).await?;
        let dbs = self.introspect(&request.connection_id, false).await?;
        let table = find_table(&dbs, &target_database, &request.schema, &request.table)?;

        let (engine, connection) = {
            let inner = self.inner.read().await;
            let open = inner.open.get(&request.connection_id).ok_or_else(|| {
                CellarError::NotConnected(format!(
                    "no open connection for {}",
                    request.connection_id
                ))
            })?;
            (open.config.engine, Arc::clone(&open.connection))
        };

        match engine {
            Engine::Firestore => {
                cellar_driver_firestore::browse_collection(connection.as_ref(), &request, &table)
                    .await
            }
            Engine::Postgres => {
                match cellar_driver_postgres::browse_table(connection.as_ref(), &request, &table)
                    .await
                {
                    Ok(result) => Ok(result),
                    Err(err) => Err(self
                        .handle_operation_error(&request.connection_id, err)
                        .await),
                }
            }
            other => Err(CellarError::invalid_config(format!(
                "engine {} does not support table browsing yet",
                other.as_str()
            ))),
        }
    }

    pub async fn commit_table_changes(
        &self,
        id: &str,
        request: TableChangeRequest,
    ) -> CellarResult<TableCommitResult> {
        let (engine, connection) = {
            let inner = self.inner.read().await;
            let open = inner
                .open
                .get(id)
                .ok_or_else(|| CellarError::NotConnected(format!("no open connection for {id}")))?;
            (open.config.engine, Arc::clone(&open.connection))
        };
        match engine {
            Engine::Postgres => {
                match cellar_driver_postgres::commit_table_changes(connection.as_ref(), &request)
                    .await
                {
                    Ok(result) => Ok(result),
                    Err(err) => Err(self.handle_operation_error(id, err).await),
                }
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
        let (engine, connection) = {
            let inner = self.inner.read().await;
            let open = inner
                .open
                .get(id)
                .ok_or_else(|| CellarError::NotConnected(format!("no open connection for {id}")))?;
            (open.config.engine, Arc::clone(&open.connection))
        };
        if engine != Engine::Postgres {
            return Err(CellarError::invalid_config(format!(
                "execution plans are not available for {} yet",
                engine.as_str()
            )));
        }
        let driver = driver_for(engine)?;
        match driver
            .explain_query(connection.as_ref(), &query, mode)
            .await
        {
            Ok(plan) => Ok(plan),
            Err(err) => Err(self.handle_operation_error(id, err).await),
        }
    }

    async fn config_for(&self, id: &str) -> CellarResult<ConnectionConfig> {
        let inner = self.inner.read().await;
        inner
            .configs
            .get(id)
            .cloned()
            .ok_or_else(|| CellarError::invalid_config(format!("unknown connection {id}")))
    }

    async fn target_database_for(&self, request: &TableBrowseRequest) -> CellarResult<String> {
        if let Some(database) = &request.database {
            if database.trim().is_empty() {
                return Err(CellarError::invalid_config("database name is empty"));
            }
            return Ok(database.clone());
        }

        let inner = self.inner.read().await;
        let open = inner.open.get(&request.connection_id).ok_or_else(|| {
            CellarError::NotConnected(format!("no open connection for {}", request.connection_id))
        })?;
        Ok(open.config.database.clone())
    }

    async fn remove_open(&self, id: &str) -> Option<OpenConnection> {
        let mut inner = self.inner.write().await;
        inner.schema_cache.remove(id);
        inner.open.remove(id)
    }

    async fn remove_open_if_same(
        &self,
        id: &str,
        connection: &Arc<dyn Connection>,
    ) -> Option<OpenConnection> {
        let mut inner = self.inner.write().await;
        let same = inner
            .open
            .get(id)
            .map(|open| Arc::ptr_eq(&open.connection, connection))
            .unwrap_or(false);
        if !same {
            return None;
        }
        inner.schema_cache.remove(id);
        inner.open.remove(id)
    }

    async fn handle_operation_error(&self, id: &str, err: CellarError) -> CellarError {
        if !should_evict_connection(&err) {
            return err;
        }
        if let Some(open) = self.remove_open(id).await {
            let _ = open.connection.close().await;
        }
        reconnectable_error(err)
    }
}

fn should_evict_connection(err: &CellarError) -> bool {
    matches!(
        err,
        CellarError::Connection(_) | CellarError::Tls(_) | CellarError::NotConnected(_)
    )
}

fn reconnectable_error(err: CellarError) -> CellarError {
    CellarError::NotConnected(format!(
        "The database connection was lost and the stale pool was closed. Reconnect and retry the action. Last error: {err}"
    ))
}

fn find_table(dbs: &[Database], database: &str, schema: &str, table: &str) -> CellarResult<Table> {
    dbs.iter()
        .find(|db| db.name == database)
        .and_then(|db| db.schemas.iter().find(|s| s.name == schema))
        .and_then(|s| s.tables.iter().find(|t| t.name == table))
        .cloned()
        .ok_or_else(|| {
            CellarError::invalid_config(format!(
                "table {database}.{schema}.{table} was not found in schema metadata"
            ))
        })
}

fn driver_for(engine: Engine) -> CellarResult<Box<dyn Driver>> {
    match engine {
        Engine::Firestore => Ok(Box::new(FirestoreDriver::new())),
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
