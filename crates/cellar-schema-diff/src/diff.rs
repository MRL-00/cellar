//! Structural comparison of two [`Schema`] trees.
//!
//! [`diff_schemas`] matches objects by name and classifies each as added,
//! removed, modified, or unchanged. The result keeps the full source and
//! target objects so the UI can render a side-by-side view; DDL generation
//! ([`crate::migration`]) consumes the same inputs separately.

use cellar_core::schema::{Column, ForeignKey, Index, Schema, Table, View};
use serde::{Deserialize, Serialize};
use specta::Type;

/// Per-object classification in a schema comparison.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ChangeStatus {
    /// Present in target but not source — a `CREATE`/`ADD` candidate.
    Added,
    /// Present in source but not target — a `DROP` candidate.
    Removed,
    /// Present in both, but the definitions differ.
    Modified,
    /// Present in both and identical.
    Unchanged,
}

impl ChangeStatus {
    /// `true` for anything other than [`ChangeStatus::Unchanged`].
    pub fn is_change(self) -> bool {
        !matches!(self, ChangeStatus::Unchanged)
    }
}

/// Top-level result of comparing a `source` schema against a `target` schema.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct SchemaDiff {
    pub source_label: String,
    pub target_label: String,
    pub source_schema: String,
    pub target_schema: String,
    pub tables: Vec<TableDiff>,
    pub views: Vec<ViewDiff>,
    pub summary: DiffSummary,
}

/// Aggregate counts, used for the comparison header without re-walking the
/// tree on the frontend.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct DiffSummary {
    pub tables_added: u32,
    pub tables_removed: u32,
    pub tables_modified: u32,
    pub tables_unchanged: u32,
    pub views_added: u32,
    pub views_removed: u32,
    pub views_modified: u32,
    pub views_unchanged: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct TableDiff {
    pub name: String,
    pub status: ChangeStatus,
    pub columns: Vec<ColumnDiff>,
    pub indexes: Vec<IndexDiff>,
    pub foreign_keys: Vec<ForeignKeyDiff>,
    pub primary_key: PrimaryKeyDiff,
    /// Full source/target objects so the UI can render either side verbatim.
    pub source: Option<Table>,
    pub target: Option<Table>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct ColumnDiff {
    pub name: String,
    pub status: ChangeStatus,
    pub source: Option<Column>,
    pub target: Option<Column>,
    /// Human-readable field-level changes when `status == Modified`
    /// (e.g. `type int4 → bigint`, `set NOT NULL`).
    pub changes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct IndexDiff {
    pub name: String,
    pub status: ChangeStatus,
    pub source: Option<Index>,
    pub target: Option<Index>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct ForeignKeyDiff {
    pub name: String,
    pub status: ChangeStatus,
    pub source: Option<ForeignKey>,
    pub target: Option<ForeignKey>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct PrimaryKeyDiff {
    pub status: ChangeStatus,
    pub source: Vec<String>,
    pub target: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct ViewDiff {
    pub name: String,
    pub status: ChangeStatus,
    pub source: Option<View>,
    pub target: Option<View>,
}

/// Compare two schemas. `source_label`/`target_label` are carried through
/// verbatim for display (e.g. `"prod / public"`); the diff itself is
/// independent of where each schema came from (live connection or snapshot).
pub fn diff_schemas(
    source: &Schema,
    target: &Schema,
    source_label: impl Into<String>,
    target_label: impl Into<String>,
) -> SchemaDiff {
    let tables = diff_tables(&source.tables, &target.tables);
    let views = diff_views(&source.views, &target.views);
    let summary = summarize(&tables, &views);

    SchemaDiff {
        source_label: source_label.into(),
        target_label: target_label.into(),
        source_schema: source.name.clone(),
        target_schema: target.name.clone(),
        tables,
        views,
        summary,
    }
}

fn diff_tables(source: &[Table], target: &[Table]) -> Vec<TableDiff> {
    let names = ordered_union(
        source.iter().map(|t| t.name.as_str()),
        target.iter().map(|t| t.name.as_str()),
    );
    names
        .into_iter()
        .map(|name| {
            let src = source.iter().find(|t| t.name == name);
            let tgt = target.iter().find(|t| t.name == name);
            diff_table(name, src, tgt)
        })
        .collect()
}

fn diff_table(name: String, source: Option<&Table>, target: Option<&Table>) -> TableDiff {
    let columns = diff_columns(
        source.map(|t| t.columns.as_slice()).unwrap_or(&[]),
        target.map(|t| t.columns.as_slice()).unwrap_or(&[]),
    );
    let indexes = diff_indexes(
        source.map(|t| t.indexes.as_slice()).unwrap_or(&[]),
        target.map(|t| t.indexes.as_slice()).unwrap_or(&[]),
    );
    let foreign_keys = diff_foreign_keys(
        source.map(|t| t.foreign_keys.as_slice()).unwrap_or(&[]),
        target.map(|t| t.foreign_keys.as_slice()).unwrap_or(&[]),
    );
    let primary_key = diff_primary_key(
        source.map(|t| t.primary_key.clone()).unwrap_or_default(),
        target.map(|t| t.primary_key.clone()).unwrap_or_default(),
    );

    let status = match (source, target) {
        (None, Some(_)) => ChangeStatus::Added,
        (Some(_), None) => ChangeStatus::Removed,
        (Some(_), Some(_)) => {
            let inner_change = columns.iter().any(|c| c.status.is_change())
                || indexes.iter().any(|i| i.status.is_change())
                || foreign_keys.iter().any(|f| f.status.is_change())
                || primary_key.status.is_change();
            if inner_change {
                ChangeStatus::Modified
            } else {
                ChangeStatus::Unchanged
            }
        }
        (None, None) => ChangeStatus::Unchanged,
    };

    TableDiff {
        name,
        status,
        columns,
        indexes,
        foreign_keys,
        primary_key,
        source: source.cloned(),
        target: target.cloned(),
    }
}

fn diff_columns(source: &[Column], target: &[Column]) -> Vec<ColumnDiff> {
    let names = ordered_union(
        source.iter().map(|c| c.name.as_str()),
        target.iter().map(|c| c.name.as_str()),
    );
    names
        .into_iter()
        .map(|name| {
            let src = source.iter().find(|c| c.name == name);
            let tgt = target.iter().find(|c| c.name == name);
            let (status, changes) = match (src, tgt) {
                (None, Some(_)) => (ChangeStatus::Added, Vec::new()),
                (Some(_), None) => (ChangeStatus::Removed, Vec::new()),
                (Some(a), Some(b)) => {
                    let changes = column_field_changes(a, b);
                    if changes.is_empty() {
                        (ChangeStatus::Unchanged, changes)
                    } else {
                        (ChangeStatus::Modified, changes)
                    }
                }
                (None, None) => (ChangeStatus::Unchanged, Vec::new()),
            };
            ColumnDiff {
                name,
                status,
                source: src.cloned(),
                target: tgt.cloned(),
                changes,
            }
        })
        .collect()
}

/// Describe how a column changed between source and target. Empty means the
/// columns are equivalent for the fields Cellar models.
fn column_field_changes(source: &Column, target: &Column) -> Vec<String> {
    let mut changes = Vec::new();
    if source.data_type != target.data_type {
        changes.push(format!("type {} → {}", source.data_type, target.data_type));
    }
    if source.nullable != target.nullable {
        changes.push(if target.nullable {
            "drop NOT NULL".to_string()
        } else {
            "set NOT NULL".to_string()
        });
    }
    if source.default != target.default {
        changes.push(match &target.default {
            Some(def) => format!("default → {def}"),
            None => "drop default".to_string(),
        });
    }
    changes
}

fn diff_indexes(source: &[Index], target: &[Index]) -> Vec<IndexDiff> {
    let names = ordered_union(
        source.iter().map(|i| i.name.as_str()),
        target.iter().map(|i| i.name.as_str()),
    );
    names
        .into_iter()
        .map(|name| {
            let src = source.iter().find(|i| i.name == name);
            let tgt = target.iter().find(|i| i.name == name);
            let status = match (src, tgt) {
                (None, Some(_)) => ChangeStatus::Added,
                (Some(_), None) => ChangeStatus::Removed,
                (Some(a), Some(b)) if a != b => ChangeStatus::Modified,
                _ => ChangeStatus::Unchanged,
            };
            IndexDiff {
                name,
                status,
                source: src.cloned(),
                target: tgt.cloned(),
            }
        })
        .collect()
}

fn diff_foreign_keys(source: &[ForeignKey], target: &[ForeignKey]) -> Vec<ForeignKeyDiff> {
    let names = ordered_union(
        source.iter().map(|f| f.name.as_str()),
        target.iter().map(|f| f.name.as_str()),
    );
    names
        .into_iter()
        .map(|name| {
            let src = source.iter().find(|f| f.name == name);
            let tgt = target.iter().find(|f| f.name == name);
            let status = match (src, tgt) {
                (None, Some(_)) => ChangeStatus::Added,
                (Some(_), None) => ChangeStatus::Removed,
                (Some(a), Some(b)) if a != b => ChangeStatus::Modified,
                _ => ChangeStatus::Unchanged,
            };
            ForeignKeyDiff {
                name,
                status,
                source: src.cloned(),
                target: tgt.cloned(),
            }
        })
        .collect()
}

fn diff_primary_key(source: Vec<String>, target: Vec<String>) -> PrimaryKeyDiff {
    let status = if source == target {
        ChangeStatus::Unchanged
    } else if source.is_empty() {
        ChangeStatus::Added
    } else if target.is_empty() {
        ChangeStatus::Removed
    } else {
        ChangeStatus::Modified
    };
    PrimaryKeyDiff {
        status,
        source,
        target,
    }
}

fn diff_views(source: &[View], target: &[View]) -> Vec<ViewDiff> {
    let names = ordered_union(
        source.iter().map(|v| v.name.as_str()),
        target.iter().map(|v| v.name.as_str()),
    );
    names
        .into_iter()
        .map(|name| {
            let src = source.iter().find(|v| v.name == name);
            let tgt = target.iter().find(|v| v.name == name);
            let status = match (src, tgt) {
                (None, Some(_)) => ChangeStatus::Added,
                (Some(_), None) => ChangeStatus::Removed,
                (Some(a), Some(b)) if a.definition != b.definition || a.columns != b.columns => {
                    ChangeStatus::Modified
                }
                _ => ChangeStatus::Unchanged,
            };
            ViewDiff {
                name,
                status,
                source: src.cloned(),
                target: tgt.cloned(),
            }
        })
        .collect()
}

fn summarize(tables: &[TableDiff], views: &[ViewDiff]) -> DiffSummary {
    let mut summary = DiffSummary::default();
    for table in tables {
        match table.status {
            ChangeStatus::Added => summary.tables_added += 1,
            ChangeStatus::Removed => summary.tables_removed += 1,
            ChangeStatus::Modified => summary.tables_modified += 1,
            ChangeStatus::Unchanged => summary.tables_unchanged += 1,
        }
    }
    for view in views {
        match view.status {
            ChangeStatus::Added => summary.views_added += 1,
            ChangeStatus::Removed => summary.views_removed += 1,
            ChangeStatus::Modified => summary.views_modified += 1,
            ChangeStatus::Unchanged => summary.views_unchanged += 1,
        }
    }
    summary
}

/// Names from both sides in a stable order: source order first, then any
/// target-only names in target order. Keeps the diff deterministic and close
/// to how each side lists its objects.
fn ordered_union<'a>(
    source: impl Iterator<Item = &'a str>,
    target: impl Iterator<Item = &'a str>,
) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for name in source {
        if !out.iter().any(|n| n == name) {
            out.push(name.to_string());
        }
    }
    for name in target {
        if !out.iter().any(|n| n == name) {
            out.push(name.to_string());
        }
    }
    out
}
