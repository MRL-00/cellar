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
