//! Schema snapshot types for offline comparison.
//!
//! A snapshot captures one database's introspected [`Database`] tree at a
//! point in time so it can be compared against a live schema (or another
//! snapshot) later. The host (`apps/desktop`) owns serializing these to
//! `~/.cellar/snapshots/`; this crate only defines the shape so both the
//! reader and the diff path agree on it.

use cellar_core::schema::Database;
use serde::{Deserialize, Serialize};
use specta::Type;

/// Lightweight descriptor listed in the snapshot picker without loading the
/// full schema tree from disk.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct SchemaSnapshotMeta {
    pub id: String,
    pub label: String,
    pub engine: String,
    pub connection_id: String,
    pub connection_name: String,
    pub database: String,
    /// Schema names captured, so the picker can offer a namespace to compare.
    pub schemas: Vec<String>,
    pub table_count: u32,
    /// Unix epoch milliseconds when the snapshot was saved.
    pub created_at_ms: i64,
}

/// A saved snapshot: its descriptor plus the captured database tree.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct SchemaSnapshot {
    pub meta: SchemaSnapshotMeta,
    pub database: Database,
}
