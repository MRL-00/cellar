use serde::{Deserialize, Serialize};
use specta::Type;

use crate::value::{ColumnMeta, Row};

/// SQL statement plus execution hints. Drivers may layer their own dialect
/// rewriting, but the host always hands them a query in this shape.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Query {
    pub sql: String,
    /// If `Some`, the driver should append (or enforce) a row cap. SPEC §5.4
    /// asks for streamed page-style results — for the first vertical slice we
    /// just return up to `max_rows` rows in one shot.
    pub max_rows: Option<u32>,
    /// Target database. For engines like Postgres where a connection is bound
    /// to one database, the driver routes the query to a pool for this
    /// database (the sidebar can browse several databases per connection).
    /// `None` means "use the connection's default database".
    pub database: Option<String>,
}

impl Query {
    pub fn new(sql: impl Into<String>) -> Self {
        Self {
            sql: sql.into(),
            max_rows: None,
            database: None,
        }
    }

    pub fn with_max_rows(mut self, max_rows: u32) -> Self {
        self.max_rows = Some(max_rows);
        self
    }

    pub fn with_database(mut self, database: impl Into<String>) -> Self {
        self.database = Some(database.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct QueryResult {
    pub columns: Vec<ColumnMeta>,
    pub rows: Vec<Row>,
    /// Server-reported affected-row count for DML, or `None` for SELECT-like
    /// statements where it isn't meaningful.
    pub rows_affected: Option<u64>,
    /// Total elapsed time for the round-trip, measured in the host.
    pub duration_ms: u64,
    /// `true` when the driver truncated the result because it hit
    /// [`Query::max_rows`]. The grid uses this to show a "+ more" badge.
    pub truncated: bool,
}
