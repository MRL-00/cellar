use serde::{Deserialize, Serialize};
use specta::Type;

use crate::value::{CellValue, ColumnMeta, Row};

/// SQL statement plus execution hints. Drivers may layer their own dialect
/// rewriting, but the host always hands them a query in this shape.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Query {
    pub sql: String,
    /// If `Some`, the driver should append (or enforce) a row cap. SPEC §5.4
    /// asks for streamed page-style results — for the first vertical slice we
    /// just return up to `max_rows` rows in one shot.
    pub max_rows: Option<u32>,
    /// Zero-based row offset for paginated queries. The driver skips this many
    /// rows before collecting up to `max_rows`. Because free-form SQL passes
    /// through verbatim (no parser to inject OFFSET), the driver implements
    /// this by consuming and discarding the leading rows from the stream — this
    /// transfers the skipped rows over the wire. For large offsets a re-issued
    /// query with a subquery wrapper would be cheaper, but that requires a
    /// parser and is deferred. Use this for "Load more" UX where the offset is
    /// small relative to `max_rows`.
    pub offset: Option<u32>,
    /// Target database. For engines like Postgres where a connection is bound
    /// to one database, the driver routes the query to a pool for this
    /// database (the sidebar can browse several databases per connection).
    /// `None` means "use the connection's default database".
    pub database: Option<String>,
    /// Caller-chosen handle for in-flight cancellation. When `Some`, drivers
    /// that support [`crate::driver::Driver::cancel_query`] register the
    /// running statement under this id so a concurrent cancel call can find
    /// it. `None` opts out of cancellation bookkeeping.
    #[serde(default)]
    pub query_id: Option<String>,
    /// Values for named (`:name`) or positional (`$N`) placeholders in `sql`.
    /// When non-empty the driver MUST bind these through the engine's native
    /// parameter protocol — never by interpolating them into the SQL text.
    /// Keyed by parameter name (the identifier after `:` for named params, the
    /// number for `$N`); the driver re-derives bind order from the SQL so it
    /// does not trust caller-supplied ordering.
    #[serde(default)]
    pub params: Vec<QueryParam>,
}

/// One bound parameter value supplied by the caller. Carries a typed
/// [`CellValue`] so the driver can bind it through the native protocol with the
/// right wire type instead of stringifying it.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
pub struct QueryParam {
    /// Parameter name without its sigil. For `:user_id` this is `user_id`; for
    /// `$1` this is `1`.
    pub name: String,
    pub value: CellValue,
}

/// Whether a detected placeholder was written as a named (`:name`) or
/// positional (`$N`) parameter.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ParameterStyle {
    Named,
    Positional,
}

/// A placeholder detected in a SQL statement. The frontend uses this to render
/// a labeled input before running the query.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct DetectedParameter {
    /// Parameter name without its sigil (`user_id` for `:user_id`, `1` for
    /// `$1`). Distinct names appear once; a name reused in the statement is
    /// reported a single time.
    pub name: String,
    /// The placeholder exactly as it appears in the SQL (`:user_id`, `$1`).
    pub placeholder: String,
    pub style: ParameterStyle,
    /// 1-based bind position in the order the distinct names first appear.
    pub ordinal: u32,
    /// Best-effort column the placeholder is compared against (`id` for
    /// `WHERE id = :id`), so the UI can infer an input type from schema.
    /// `None` when no simple comparison was detected.
    pub column_hint: Option<String>,
}

impl Query {
    pub fn new(sql: impl Into<String>) -> Self {
        Self {
            sql: sql.into(),
            max_rows: None,
            offset: None,
            database: None,
            query_id: None,
            params: Vec::new(),
        }
    }

    pub fn with_max_rows(mut self, max_rows: u32) -> Self {
        self.max_rows = Some(max_rows);
        self
    }

    pub fn with_offset(mut self, offset: u32) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn with_database(mut self, database: impl Into<String>) -> Self {
        self.database = Some(database.into());
        self
    }

    pub fn with_query_id(mut self, query_id: impl Into<String>) -> Self {
        self.query_id = Some(query_id.into());
        self
    }

    pub fn with_params(mut self, params: Vec<QueryParam>) -> Self {
        self.params = params;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct QueryResult {
    pub columns: Vec<ColumnMeta>,
    pub rows: Vec<Row>,
    /// Database-emitted notices captured while executing this query. Host-side
    /// warnings, validation messages, and truncation badges belong in the
    /// Messages panel, not here.
    pub notices: Vec<DatabaseNotice>,
    /// Whether the current driver path can observe server notice frames.
    pub notice_capture: NoticeCapture,
    /// Server-reported affected-row count for DML, or `None` for SELECT-like
    /// statements where it isn't meaningful.
    pub rows_affected: Option<u64>,
    /// Total elapsed time for the round-trip, measured in the host.
    pub duration_ms: u64,
    /// `true` when the driver truncated the result because it hit
    /// [`Query::max_rows`]. The grid uses this to show a "+ more" badge.
    pub truncated: bool,
    /// Total row count for the underlying dataset, when the driver can provide
    /// it cheaply. `None` means "unknown" — the UI shows `truncated` alone.
    /// For table browse with `include_total: true` the driver runs a
    /// `SELECT count(*)` with the same filter clauses and returns it here.
    /// Free-form queries always return `None`.
    pub total_rows: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NoticeSeverity {
    Panic,
    Fatal,
    Error,
    Warning,
    Notice,
    Info,
    Log,
    Debug,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct DatabaseNotice {
    pub severity: NoticeSeverity,
    /// Engine-native code when available. For Postgres this is the SQLSTATE.
    pub code: Option<String>,
    pub message: String,
    pub detail: Option<String>,
    pub hint: Option<String>,
    /// RFC 3339 timestamp generated by the host when it observes the notice.
    pub timestamp: String,
    pub connection_id: Option<String>,
    pub database: Option<String>,
    pub query_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct NoticeCapture {
    pub supported: bool,
    pub reason: Option<String>,
}

impl NoticeCapture {
    pub fn supported() -> Self {
        Self {
            supported: true,
            reason: None,
        }
    }

    pub fn unsupported(reason: impl Into<String>) -> Self {
        Self {
            supported: false,
            reason: Some(reason.into()),
        }
    }
}

/// Whether an execution plan should only estimate the plan or run the
/// statement to collect actual timings. `Analyze` is intentionally explicit:
/// Postgres `EXPLAIN ANALYZE` executes the supplied SQL.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PlanMode {
    Estimate,
    Analyze,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
pub struct QueryPlan {
    pub mode: PlanMode,
    pub engine: String,
    pub database: Option<String>,
    pub sql: String,
    pub root: PlanNode,
    pub planning_time_ms: Option<f64>,
    pub execution_time_ms: Option<f64>,
    pub duration_ms: u64,
    /// Original engine plan payload for advanced inspection and future richer
    /// renderers. The typed tree above is the stable UI contract.
    pub raw_json: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
pub struct PlanNode {
    pub node_type: String,
    pub relation_name: Option<String>,
    pub schema_name: Option<String>,
    pub alias: Option<String>,
    pub index_name: Option<String>,
    pub join_type: Option<String>,
    pub startup_cost: Option<f64>,
    pub total_cost: Option<f64>,
    pub plan_rows: Option<u64>,
    pub plan_width: Option<u64>,
    pub actual_startup_time_ms: Option<f64>,
    pub actual_total_time_ms: Option<f64>,
    pub actual_rows: Option<f64>,
    pub actual_loops: Option<u64>,
    pub details: Vec<PlanDetail>,
    pub children: Vec<PlanNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct PlanDetail {
    pub label: String,
    pub value: String,
}

/// Typed request for browsing one table through the grid. This deliberately
/// does not carry executable SQL; each driver owns safe dialect-specific
/// rendering and value binding.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct TableBrowseRequest {
    pub connection_id: String,
    pub database: Option<String>,
    pub schema: String,
    pub table: String,
    pub limit: Option<u32>,
    /// Zero-based row offset for page-style table browsing. Drivers should
    /// combine this with deterministic ordering when available so table tabs
    /// stay stable as users move between pages.
    pub offset: Option<u32>,
    pub sorts: Vec<TableSortClause>,
    pub filters: Vec<TableFilterClause>,
    /// When no explicit sort is requested, order by the table primary key if
    /// metadata is available. This keeps table tabs stable without the UI
    /// building an `ORDER BY` string.
    pub primary_key_fallback_ordering: bool,
    /// When `true`, the driver additionally runs `SELECT count(*)` with the
    /// same filter clauses and returns the result in `QueryResult.total_rows`.
    /// Defaults to `false` to avoid the extra round-trip on every page flip.
    /// Callers should set this on the first page load of a table tab.
    #[serde(default)]
    pub include_total: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct TableSortClause {
    pub column: String,
    pub direction: SortDirection,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SortDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct TableFilterClause {
    pub column: String,
    pub operator: TableFilterOperator,
    /// User-entered scalar value. Null checks intentionally use operators
    /// instead of overloading this field.
    pub value: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TableFilterOperator {
    Equals,
    NotEquals,
    Contains,
    NotContains,
    StartsWith,
    EndsWith,
    /// Raw SQL LIKE pattern (`%`/`_` supplied by the user), matched case-insensitively.
    Like,
    IsNull,
    IsNotNull,
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
}
