//! Postgres driver for Cellar, built on `sqlx`. Implements [`Driver`] for the
//! Postgres engine: pooled connect, system-catalog introspection, and
//! materialized query execution with type-aware cell decoding.

use async_trait::async_trait;
use cellar_core::driver::{Connection, ConnectionConfig, Driver, Engine};
use cellar_core::error::CellarResult;
use cellar_core::query::{PlanMode, Query, QueryPlan, QueryResult, TableBrowseRequest};
use cellar_core::schema::{Database, Table};
use cellar_diff::{TableChangeRequest, TableCommitResult};

mod bind;
mod connect;
mod decode;
mod introspect;
mod query;
mod table_browse;

pub use connect::{open_pool, PgConnection};

/// Zero-sized handle. Construct once per process and reuse — drivers carry no
/// per-connection state themselves (the pool lives on [`PgConnection`]).
#[derive(Debug, Default, Clone, Copy)]
pub struct PostgresDriver;

impl PostgresDriver {
    pub fn new() -> Self {
        Self
    }
}

pub async fn commit_table_changes(
    conn: &dyn Connection,
    request: &TableChangeRequest,
) -> CellarResult<TableCommitResult> {
    let pg = connect::as_pg(conn)?;
    query::commit_table_changes(pg, request).await
}

pub async fn apply_migration(
    conn: &dyn Connection,
    database: &str,
    sql: &str,
) -> CellarResult<u64> {
    let pg = connect::as_pg(conn)?;
    query::apply_migration(pg, database, sql).await
}

pub async fn browse_table(
    conn: &dyn Connection,
    request: &TableBrowseRequest,
    table: &Table,
) -> CellarResult<QueryResult> {
    let pg = connect::as_pg(conn)?;
    table_browse::browse_table(pg, request, table).await
}

#[async_trait]
impl Driver for PostgresDriver {
    fn engine(&self) -> Engine {
        Engine::Postgres
    }

    async fn connect(
        &self,
        config: &ConnectionConfig,
        password: Option<&str>,
    ) -> CellarResult<Box<dyn Connection>> {
        let pg = open_pool(config, password).await?;
        Ok(Box::new(pg))
    }

    async fn introspect(&self, conn: &dyn Connection) -> CellarResult<Vec<Database>> {
        let pg = connect::as_pg(conn)?;
        introspect::introspect(pg).await
    }

    async fn execute_query(&self, conn: &dyn Connection, q: &Query) -> CellarResult<QueryResult> {
        let pg = connect::as_pg(conn)?;
        query::execute_query(pg, q).await
    }

    async fn explain_query(
        &self,
        conn: &dyn Connection,
        q: &Query,
        mode: PlanMode,
    ) -> CellarResult<QueryPlan> {
        let pg = connect::as_pg(conn)?;
        query::explain_query(pg, q, mode).await
    }

    async fn cancel_query(&self, conn: &dyn Connection, query_id: &str) -> CellarResult<bool> {
        let pg = connect::as_pg(conn)?;
        query::cancel_query(pg, query_id).await
    }
}
