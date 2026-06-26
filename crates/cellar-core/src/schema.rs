use serde::{Deserialize, Serialize};
use specta::Type;

/// One database visible to the connected user. For Postgres this maps to a
/// `pg_database` row, for MySQL to a single catalog, for SQLite the file.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct Database {
    pub name: String,
    pub is_default: bool,
    pub schemas: Vec<Schema>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct Schema {
    pub name: String,
    pub tables: Vec<Table>,
    pub views: Vec<View>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct Table {
    pub name: String,
    pub schema: String,
    /// `None` until lazy row-count introspection has run.
    pub row_count: Option<u64>,
    pub columns: Vec<Column>,
    pub primary_key: Vec<String>,
    pub foreign_keys: Vec<ForeignKey>,
    pub indexes: Vec<Index>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct View {
    pub name: String,
    pub schema: String,
    pub columns: Vec<Column>,
    pub definition: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct Column {
    pub name: String,
    /// The engine-native type name as reported by the catalog (e.g. `int4`,
    /// `varchar(64)`). Drivers must not normalize this — the UI relies on the
    /// raw type to render badges and pick the right editor.
    pub data_type: String,
    pub nullable: bool,
    pub default: Option<String>,
    pub is_primary_key: bool,
    pub ordinal: u32,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct ForeignKey {
    pub name: String,
    pub columns: Vec<String>,
    pub referenced_schema: String,
    pub referenced_table: String,
    pub referenced_columns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct Index {
    pub name: String,
    pub columns: Vec<String>,
    pub unique: bool,
    pub primary: bool,
}

/// What kind of database object references a searched table or column. Returned
/// by the `find_usages` command. See SPEC §6.2 (schema navigation).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UsageKind {
    View,
    MaterializedView,
    Function,
    Procedure,
    Trigger,
    Constraint,
}

/// A confirmed reference to the searched table/column found inside a view
/// definition, routine body, trigger definition, or constraint. The reference
/// is structurally confirmed by `cellar-sql` (real identifier, not a substring
/// match) before it ever reaches this type.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct UsageReference {
    pub kind: UsageKind,
    /// Schema the referencing object lives in.
    pub schema: String,
    /// Name of the referencing object (view / function / trigger / constraint).
    pub name: String,
    /// For triggers and constraints, the table the object is attached to.
    pub on_table: Option<String>,
    /// 1-based line within `definition` where the reference was found.
    pub line: u32,
    /// The matching line, trimmed for display.
    pub snippet: String,
    /// The column matched, when the search was column-scoped.
    pub matched_column: Option<String>,
    /// Full object definition so the UI can open it in an editor tab.
    pub definition: String,
}

/// A single object definition pulled from the system catalogs, cached by the
/// host so repeated `find_usages` searches don't re-query the catalogs. This is
/// host-internal and never crosses IPC, so it carries no serde/specta derives.
#[derive(Debug, Clone)]
pub struct UsageDefinition {
    pub kind: UsageKind,
    pub schema: String,
    pub name: String,
    pub on_table: Option<String>,
    pub definition: String,
}
