//! SQL Server and Azure SQL driver for Cellar, built on `tiberius`.
//!
//! Azure SQL Database and Managed Instance speak the SQL Server TDS protocol,
//! so the first slice shares one implementation and varies the exposed engine
//! plus TLS defaults.

use async_trait::async_trait;
use cellar_core::driver::{Connection, ConnectionConfig, Driver, Engine};
use cellar_core::error::{CellarError, CellarResult};
use cellar_core::query::{PlanMode, Query, QueryPlan, QueryResult, TableBrowseRequest};
use cellar_core::schema::{Database, Table};

mod connect;
mod decode;
mod introspect;
mod query;
mod table_browse;

pub use connect::{open_client, SqlServerConnection};

#[derive(Debug, Clone, Copy)]
pub struct SqlServerDriver {
    engine: Engine,
}

impl SqlServerDriver {
    pub fn new() -> Self {
        Self {
            engine: Engine::Mssql,
        }
    }

    pub fn azure() -> Self {
        Self {
            engine: Engine::Azure,
        }
    }
}

impl Default for SqlServerDriver {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn browse_table(
    conn: &dyn Connection,
    request: &TableBrowseRequest,
    table: &Table,
) -> CellarResult<QueryResult> {
    let sql = connect::as_sqlserver(conn)?;
    table_browse::browse_table(sql, request, table).await
}

#[async_trait]
impl Driver for SqlServerDriver {
    fn engine(&self) -> Engine {
        self.engine
    }

    async fn connect(
        &self,
        config: &ConnectionConfig,
        password: Option<&str>,
    ) -> CellarResult<Box<dyn Connection>> {
        let conn = open_client(config, password, self.engine).await?;
        Ok(Box::new(conn))
    }

    async fn introspect(&self, conn: &dyn Connection) -> CellarResult<Vec<Database>> {
        let sql = connect::as_sqlserver(conn)?;
        introspect::introspect(sql).await
    }

    async fn execute_query(&self, conn: &dyn Connection, q: &Query) -> CellarResult<QueryResult> {
        let sql = connect::as_sqlserver(conn)?;
        query::execute_query(sql, q).await
    }

    async fn explain_query(
        &self,
        _conn: &dyn Connection,
        _q: &Query,
        _mode: PlanMode,
    ) -> CellarResult<QueryPlan> {
        Err(CellarError::invalid_config(
            "SQL Server execution plans are not available yet",
        ))
    }
}
