use std::any::Any;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::CellarResult;
use crate::query::{PlanMode, Query, QueryPlan, QueryResult};
use crate::schema::Database;

/// User-facing identifier for which driver to load. Lives in connection
/// configs and crosses IPC, so it has to be serializable.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Engine {
    Postgres,
    MySql,
    Sqlite,
    Mssql,
    Azure,
    Firestore,
}

impl Engine {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Postgres => "postgres",
            Self::MySql => "mysql",
            Self::Sqlite => "sqlite",
            Self::Mssql => "mssql",
            Self::Azure => "azure",
            Self::Firestore => "firestore",
        }
    }
}

/// Environment tag, per SPEC §6.1. Production-tagged connections get red
/// styling and confirmation guardrails — this PR wires the data, not the UX.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EnvTag {
    Local,
    Dev,
    Staging,
    Prod,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SslMode {
    Disable,
    Prefer,
    Require,
    VerifyCa,
    VerifyFull,
}

impl Default for SslMode {
    fn default() -> Self {
        Self::Prefer
    }
}

/// Inputs required to open a connection. Passwords never live here — they
/// come out of [`cellar-secrets`](../cellar_secrets) at connect time.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ConnectionConfig {
    pub id: String,
    pub name: String,
    pub engine: Engine,
    pub host: String,
    pub port: u16,
    pub database: String,
    pub user: String,
    pub ssl_mode: SslMode,
    pub env_tag: Option<EnvTag>,
    pub application_name: Option<String>,
    /// Color swatch used by the sidebar accent strip; opaque hex like `#4f8ff7`.
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DriverInfo {
    pub engine: Engine,
    /// Server version as reported by the engine, e.g. `PostgreSQL 16.2 on
    /// x86_64-linux-gnu`.
    pub version: String,
}

/// Opaque connection handle. Drivers own the underlying pool; the host stores
/// boxed handles in `ConnectionRegistry`.
#[async_trait]
pub trait Connection: Send + Sync + Any {
    fn info(&self) -> &DriverInfo;
    async fn ping(&self) -> CellarResult<()>;
    async fn close(&self) -> CellarResult<()>;

    /// Escape hatch so drivers can downcast their own concrete handle from a
    /// `&dyn Connection`. The host always pairs the right driver with the
    /// right connection — this avoids unsafe pointer casts in driver crates.
    fn as_any(&self) -> &dyn Any;
}

/// The contract every engine driver implements. SPEC §5.3.
///
/// Async returns go through `async-trait` for object-safety: the host stores
/// drivers as `Box<dyn Driver>` so it can pick one at runtime per connection.
#[async_trait]
pub trait Driver: Send + Sync {
    fn engine(&self) -> Engine;

    /// Open a pooled connection. Implementations should cap the pool at four
    /// in the first slice — wire pool sizing through `ConnectionConfig` once
    /// SPEC §6.1 grows it.
    async fn connect(
        &self,
        config: &ConnectionConfig,
        password: Option<&str>,
    ) -> CellarResult<Box<dyn Connection>>;

    /// Walk system catalogs and return the database → schema → table tree.
    /// Implementations should cache nothing — the host owns caching.
    async fn introspect(&self, conn: &dyn Connection) -> CellarResult<Vec<Database>>;

    /// Run a query and return the materialized result. Streaming lands later.
    async fn execute_query(
        &self,
        conn: &dyn Connection,
        query: &Query,
    ) -> CellarResult<QueryResult>;

    /// Return a structured execution plan. `PlanMode::Estimate` must not run
    /// the supplied statement; `PlanMode::Analyze` may execute it and callers
    /// must gate that path explicitly in the UI.
    async fn explain_query(
        &self,
        conn: &dyn Connection,
        query: &Query,
        mode: PlanMode,
    ) -> CellarResult<QueryPlan>;
}
