//! SQLite driver for Cellar, built on `sqlx`. Implements [`Driver`] for the
//! SQLite engine: pooled connect against a database file, pragma-based
//! introspection, and materialized query execution with type-aware cell
//! decoding. `ConnectionConfig.database` holds the file path; host, port,
//! user, and password are unused.

use async_trait::async_trait;
use cellar_core::driver::{Connection, ConnectionConfig, Driver, Engine};
use cellar_core::error::CellarResult;
use cellar_core::query::{
    PlanMode, Query, QueryPlan, QueryResult, QueryResultPage, QueryResultSummary,
    TableBrowseRequest,
};
use cellar_core::schema::{Database, Table};

mod connect;
mod decode;
mod introspect;
mod query;
mod table_browse;

pub use connect::{open_pool, SqliteConnection};

/// Zero-sized handle. Construct once per process and reuse.
#[derive(Debug, Default, Clone, Copy)]
pub struct SqliteDriver;

pub async fn browse_table(
    conn: &dyn Connection,
    request: &TableBrowseRequest,
    table: &Table,
) -> CellarResult<QueryResult> {
    let sqlite = connect::as_sqlite(conn)?;
    table_browse::browse_table(sqlite, request, table).await
}

#[async_trait]
impl Driver for SqliteDriver {
    fn engine(&self) -> Engine {
        Engine::Sqlite
    }

    async fn connect(
        &self,
        config: &ConnectionConfig,
        password: Option<&str>,
    ) -> CellarResult<Box<dyn Connection>> {
        let _ = password; // SQLite files are not password-protected.
        let sqlite = open_pool(config).await?;
        Ok(Box::new(sqlite))
    }

    async fn introspect(&self, conn: &dyn Connection) -> CellarResult<Vec<Database>> {
        let sqlite = connect::as_sqlite(conn)?;
        introspect::introspect(sqlite).await
    }

    async fn execute_query(&self, conn: &dyn Connection, q: &Query) -> CellarResult<QueryResult> {
        let sqlite = connect::as_sqlite(conn)?;
        query::execute_query(sqlite, q).await
    }

    async fn execute_query_stream(
        &self,
        conn: &dyn Connection,
        q: &Query,
        page_size: usize,
        on_page: &mut (dyn FnMut(QueryResultPage) -> CellarResult<()> + Send),
    ) -> CellarResult<QueryResultSummary> {
        let sqlite = connect::as_sqlite(conn)?;
        query::execute_query_stream(sqlite, q, page_size, on_page).await
    }

    async fn explain_query(
        &self,
        _conn: &dyn Connection,
        _q: &Query,
        _mode: PlanMode,
    ) -> CellarResult<QueryPlan> {
        Err(cellar_core::error::CellarError::invalid_config(
            "SQLite execution plans are not available yet",
        ))
    }
}
