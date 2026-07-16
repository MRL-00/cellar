//! Azure Cosmos DB (NoSQL / Core SQL API) driver for Cellar.
//!
//! This first slice treats Cosmos containers as read-only tables under a
//! `documents` schema, mirroring the Firestore and Convex drivers. Columns are
//! inferred from a small document sample. Auth is the account primary key via
//! the REST data-plane HMAC signature — Entra ID is a follow-up.

use std::time::Duration;

use async_trait::async_trait;
use cellar_core::driver::{Connection, ConnectionConfig, Driver, Engine};
use cellar_core::error::{CellarError, CellarResult};
use cellar_core::query::{PlanMode, Query, QueryPlan, QueryResult, TableBrowseRequest};
use cellar_core::schema::{Database, Table};

mod auth;
mod connection;
mod mapping;
mod query;

pub use connection::CosmosConnection;

pub(crate) const SCHEMA_NAME: &str = "documents";
pub(crate) const DEFAULT_SAMPLE_SIZE: usize = 25;
pub(crate) const DEFAULT_BROWSE_LIMIT: u32 = 500;
pub(crate) const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
/// Stable REST API version with cross-partition query support.
pub(crate) const API_VERSION: &str = "2018-12-31";

#[derive(Debug, Default, Clone, Copy)]
pub struct CosmosDriver;

pub async fn browse_table(
    conn: &dyn Connection,
    request: &TableBrowseRequest,
    table: &Table,
) -> CellarResult<QueryResult> {
    let cx = as_cosmos(conn)?;
    cx.browse_table(request, table).await
}

#[async_trait]
impl Driver for CosmosDriver {
    fn engine(&self) -> Engine {
        Engine::Cosmos
    }

    async fn connect(
        &self,
        config: &ConnectionConfig,
        secret: Option<&str>,
    ) -> CellarResult<Box<dyn Connection>> {
        let connection = CosmosConnection::open(config, secret).await?;
        Ok(Box::new(connection))
    }

    async fn introspect(&self, conn: &dyn Connection) -> CellarResult<Vec<Database>> {
        let cx = as_cosmos(conn)?;
        cx.introspect().await
    }

    async fn execute_query(
        &self,
        _conn: &dyn Connection,
        _query: &Query,
    ) -> CellarResult<QueryResult> {
        Err(CellarError::query(
            "Cosmos SQL query execution is not supported yet; browse containers from the sidebar",
        ))
    }

    async fn explain_query(
        &self,
        _conn: &dyn Connection,
        _query: &Query,
        _mode: PlanMode,
    ) -> CellarResult<QueryPlan> {
        Err(CellarError::invalid_config(
            "Cosmos does not expose SQL execution plans through Cellar yet",
        ))
    }
}

fn as_cosmos<'a>(conn: &'a dyn Connection) -> CellarResult<&'a CosmosConnection> {
    conn.as_any()
        .downcast_ref::<CosmosConnection>()
        .ok_or_else(|| {
            CellarError::NotConnected(format!(
                "expected cosmos connection, got {}",
                conn.info().engine.as_str()
            ))
        })
}
