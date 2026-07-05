use std::collections::HashMap;
use std::sync::Arc;

use cellar_core::driver::{Connection, ConnectionConfig, DriverInfo, Engine};
use cellar_core::error::{CellarError, CellarResult};
use cellar_core::query::{PlanMode, Query, QueryPlan, QueryResult, TableBrowseRequest};
use cellar_core::schema::{Database, Schema, UsageDefinition, UsageReference};
use cellar_diff::{TableChangeRequest, TableCommitResult};
use tokio::fs;
use tokio::sync::RwLock;

mod support;
#[cfg(test)]
mod tests;

pub(crate) use support::cellar_dir;
use support::{
    driver_for, find_table, persist, reconnectable_error, should_evict_connection, storage_path,
};

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
    /// "Find Usages" object definitions, keyed by (connection id, database).
    /// Populated lazily on the first search and dropped whenever the schema is
    /// refreshed or the connection is torn down, so it never goes stale.
    usage_cache: HashMap<(String, String), Vec<UsageDefinition>>,
}

impl RegistryInner {
    /// Drop every cached entry tied to a connection: schema tree and the
    /// per-database usage definitions. Called whenever the connection's schema
    /// is refreshed or the connection is closed.
    fn invalidate_connection(&mut self, id: &str) {
        self.schema_cache.remove(id);
        self.usage_cache.retain(|(conn, _), _| conn != id);
    }
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
                usage_cache: HashMap::new(),
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
        // Build the new configs map, persist to disk FIRST, then update memory.
        // This prevents in-memory state from diverging from disk if persist fails.
        {
            let mut inner = self.inner.write().await;
            let mut new_configs = inner.configs.clone();
            new_configs.insert(config.id.clone(), config.clone());
            persist(&new_configs).await?;
            inner.configs = new_configs;
        }
        Ok(config)
    }

    pub async fn delete(&self, id: &str) -> CellarResult<()> {
        // Compute the post-delete configs map, persist it FIRST, then mutate
        // in-memory state (including tearing down the open connection and cache).
        // If persist fails the in-memory state is left unchanged so the registry
        // stays consistent with what is on disk.
        let open_to_close = {
            let mut inner = self.inner.write().await;
            let mut new_configs = inner.configs.clone();
            new_configs.remove(id);
            persist(&new_configs).await?;
            // Persist succeeded — safe to update in-memory state now.
            inner.configs = new_configs;
            let open = inner.open.remove(id);
            inner.invalidate_connection(id);
            open
        };
        if let Some(open) = open_to_close {
            // Best-effort close; ignore errors when tearing down.
            let _ = open.connection.close().await;
        }
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
        inner.invalidate_connection(id);
        drop(inner);
        if let Some(open) = removed {
            let _ = open.connection.close().await;
        }
        Ok(())
    }

    pub async fn introspect(&self, id: &str, refresh: bool) -> CellarResult<Vec<Database>> {
        if refresh {
            // A manual refresh must also drop the cached usage definitions so
            // "Find Usages" reflects the new schema (SPEC: invalidate on refresh).
            let mut inner = self.inner.write().await;
            inner.usage_cache.retain(|(conn, _), _| conn != id);
        } else {
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

    /// Best-effort cancel of a query started with a `query_id`. Failures pass
    /// through verbatim — a failed cancel must not mark the connection broken
    /// (the original query is still running and reports its own outcome).
    pub async fn cancel_query(&self, id: &str, query_id: &str) -> CellarResult<bool> {
        let (engine, connection) = {
            let inner = self.inner.read().await;
            let open = inner
                .open
                .get(id)
                .ok_or_else(|| CellarError::NotConnected(format!("no open connection for {id}")))?;
            (open.config.engine, Arc::clone(&open.connection))
        };
        let driver = driver_for(engine)?;
        driver.cancel_query(connection.as_ref(), query_id).await
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

        // Dispatch on family so hosted providers (Supabase/Neon/PlanetScale)
        // browse through their base engine's driver.
        match engine.family() {
            Engine::Firestore => {
                cellar_driver_firestore::browse_collection(connection.as_ref(), &request, &table)
                    .await
            }
            Engine::Convex => {
                cellar_driver_convex::browse_table(connection.as_ref(), &request, &table).await
            }
            Engine::MySql => {
                match cellar_driver_mysql::browse_table(connection.as_ref(), &request, &table).await
                {
                    Ok(result) => Ok(result),
                    Err(err) => Err(self
                        .handle_operation_error(&request.connection_id, err)
                        .await),
                }
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
            Engine::Sqlite => {
                match cellar_driver_sqlite::browse_table(connection.as_ref(), &request, &table)
                    .await
                {
                    Ok(result) => Ok(result),
                    Err(err) => Err(self
                        .handle_operation_error(&request.connection_id, err)
                        .await),
                }
            }
            Engine::Mssql => {
                match cellar_driver_sqlserver::browse_table(connection.as_ref(), &request, &table)
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

    /// Find views, routines, triggers, and constraints that reference a table
    /// or column. Catalog definitions are fetched once per connection+database
    /// and cached (invalidated on schema refresh); the search itself runs over
    /// the cache and confirms each hit structurally via `cellar-sql`.
    pub async fn find_usages(
        &self,
        id: &str,
        database: Option<String>,
        schema: String,
        object_name: String,
        column_name: Option<String>,
        all_schemas: bool,
    ) -> CellarResult<Vec<UsageReference>> {
        let (engine, connection, default_db) = {
            let inner = self.inner.read().await;
            let open = inner
                .open
                .get(id)
                .ok_or_else(|| CellarError::NotConnected(format!("no open connection for {id}")))?;
            (
                open.config.engine,
                Arc::clone(&open.connection),
                open.config.database.clone(),
            )
        };

        if engine.family() != Engine::Postgres {
            return Err(CellarError::invalid_config(format!(
                "find usages is not available for {} yet",
                engine.as_str()
            )));
        }

        let target_db = database
            .filter(|d| !d.trim().is_empty())
            .unwrap_or(default_db);
        let key = (id.to_string(), target_db.clone());

        let cached = {
            let inner = self.inner.read().await;
            inner.usage_cache.get(&key).cloned()
        };
        let defs = match cached {
            Some(defs) => defs,
            None => {
                let fetched = match cellar_driver_postgres::fetch_usage_definitions(
                    connection.as_ref(),
                    &target_db,
                )
                .await
                {
                    Ok(defs) => defs,
                    Err(err) => return Err(self.handle_operation_error(id, err).await),
                };
                let mut w = self.inner.write().await;
                w.usage_cache.insert(key, fetched.clone());
                fetched
            }
        };

        let schema_filter = if all_schemas {
            None
        } else {
            Some(schema.as_str())
        };
        Ok(cellar_driver_postgres::search_usages(
            &defs,
            schema_filter,
            &schema,
            &object_name,
            column_name.as_deref(),
        ))
    }

    pub async fn commit_table_changes(
        &self,
        id: &str,
        request: TableChangeRequest,
    ) -> CellarResult<TableCommitResult> {
        self.run_table_commit(id, request, false).await
    }

    pub async fn commit_table_import(
        &self,
        id: &str,
        request: TableChangeRequest,
    ) -> CellarResult<TableCommitResult> {
        self.run_table_commit(id, request, true).await
    }

    async fn run_table_commit(
        &self,
        id: &str,
        request: TableChangeRequest,
        import: bool,
    ) -> CellarResult<TableCommitResult> {
        let (engine, connection) = {
            let inner = self.inner.read().await;
            let open = inner
                .open
                .get(id)
                .ok_or_else(|| CellarError::NotConnected(format!("no open connection for {id}")))?;
            (open.config.engine, Arc::clone(&open.connection))
        };
        match engine.family() {
            Engine::Postgres => {
                let conn = connection.as_ref();
                let outcome = if import {
                    cellar_driver_postgres::commit_table_import(conn, &request).await
                } else {
                    cellar_driver_postgres::commit_table_changes(conn, &request).await
                };
                match outcome {
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

    /// Resolve one schema namespace from a connection's live tree. Forces a
    /// fresh introspection so schema comparison (and "Recompare") reflects any
    /// external DDL since the tree was last cached.
    pub async fn schema_for(&self, id: &str, database: &str, schema: &str) -> CellarResult<Schema> {
        let dbs = self.introspect(id, true).await?;
        dbs.iter()
            .find(|db| db.name == database)
            .and_then(|db| db.schemas.iter().find(|s| s.name == schema))
            .cloned()
            .ok_or_else(|| {
                CellarError::invalid_config(format!(
                    "schema {database}.{schema} was not found on connection {id}"
                ))
            })
    }

    /// Resolve a whole database tree from a connection, for snapshot capture.
    /// Forces a fresh introspection so the snapshot records current schema.
    pub async fn database_for(&self, id: &str, database: &str) -> CellarResult<Database> {
        let dbs = self.introspect(id, true).await?;
        dbs.into_iter()
            .find(|db| db.name == database)
            .ok_or_else(|| {
                CellarError::invalid_config(format!(
                    "database {database} was not found on connection {id}"
                ))
            })
    }

    /// The engine for a saved or open connection, if Cellar knows about it.
    pub async fn engine_for(&self, id: &str) -> Option<Engine> {
        let inner = self.inner.read().await;
        inner
            .configs
            .get(id)
            .map(|c| c.engine)
            .or_else(|| inner.open.get(id).map(|o| o.config.engine))
    }

    /// Apply a reviewed schema migration script against `database` on the open
    /// connection `id`. Engine-gated to drivers that support it; the script is
    /// executed verbatim (transaction wrapping is part of the script itself).
    pub async fn apply_migration(&self, id: &str, database: &str, sql: &str) -> CellarResult<u64> {
        let (engine, connection) = {
            let inner = self.inner.read().await;
            let open = inner
                .open
                .get(id)
                .ok_or_else(|| CellarError::NotConnected(format!("no open connection for {id}")))?;
            (open.config.engine, Arc::clone(&open.connection))
        };
        match engine.family() {
            Engine::Postgres => {
                match cellar_driver_postgres::apply_migration(connection.as_ref(), database, sql)
                    .await
                {
                    Ok(duration) => Ok(duration),
                    Err(err) => Err(self.handle_operation_error(id, err).await),
                }
            }
            other => Err(CellarError::invalid_config(format!(
                "engine {} does not support schema migrations yet",
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
        if engine.family() != Engine::Postgres {
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
        inner.invalidate_connection(id);
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
        inner.invalidate_connection(id);
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
