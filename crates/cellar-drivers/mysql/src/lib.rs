//! MySQL driver for Cellar, built on `sqlx`. Implements [`Driver`] for the
//! MySQL engine: pooled connect, `information_schema` introspection, and
//! materialized query execution with type-aware cell decoding.

use async_trait::async_trait;
use cellar_core::driver::{Connection, ConnectionConfig, Driver, Engine};
use cellar_core::error::CellarResult;
use cellar_core::query::{PlanMode, Query, QueryPlan, QueryResult, TableBrowseRequest};
use cellar_core::schema::{Database, Table};

mod connect;
mod decode;
mod introspect;
mod query;
mod table_browse;

pub use connect::{open_pool, MySqlConnection};

/// Zero-sized handle. Construct once per process and reuse.
#[derive(Debug, Default, Clone, Copy)]
pub struct MySqlDriver;

pub async fn browse_table(
    conn: &dyn Connection,
    request: &TableBrowseRequest,
    table: &Table,
) -> CellarResult<QueryResult> {
    let mysql = connect::as_mysql(conn)?;
    table_browse::browse_table(mysql, request, table).await
}

#[async_trait]
impl Driver for MySqlDriver {
    fn engine(&self) -> Engine {
        Engine::MySql
    }

    async fn connect(
        &self,
        config: &ConnectionConfig,
        password: Option<&str>,
    ) -> CellarResult<Box<dyn Connection>> {
        let mysql = open_pool(config, password).await?;
        Ok(Box::new(mysql))
    }

    async fn introspect(&self, conn: &dyn Connection) -> CellarResult<Vec<Database>> {
        let mysql = connect::as_mysql(conn)?;
        introspect::introspect(mysql).await
    }

    async fn execute_query(&self, conn: &dyn Connection, q: &Query) -> CellarResult<QueryResult> {
        let mysql = connect::as_mysql(conn)?;
        query::execute_query(mysql, q).await
    }

    async fn explain_query(
        &self,
        _conn: &dyn Connection,
        _q: &Query,
        _mode: PlanMode,
    ) -> CellarResult<QueryPlan> {
        Err(cellar_core::error::CellarError::invalid_config(
            "MySQL execution plans are not available yet",
        ))
    }
}
