//! Build reviewable, transactional SQL from grid pending changes.

use serde::{Deserialize, Serialize};
use specta::Type;
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DiffError {
    #[error("table name is empty")]
    EmptyTable,
    #[error("schema name is empty")]
    EmptySchema,
    #[error("primary key metadata is required to commit row edits")]
    MissingPrimaryKey,
    #[error("change {0} does not include primary key values")]
    MissingRowKey(String),
    #[error("change {0} does not edit any columns")]
    EmptyEdit(String),
    #[error("insert change {0} does not include any values")]
    EmptyInsert(String),
    #[error("upsert change {0} does not name any conflict-target columns")]
    MissingConflictTarget(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct TableChangeRequest {
    pub database: Option<String>,
    pub schema: String,
    pub table: String,
    pub primary_key: Vec<String>,
    pub columns: Vec<DiffColumn>,
    pub changes: Vec<RowChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct DiffColumn {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum RowChange {
    Update {
        row_id: String,
        keys: Vec<CellAssignment>,
        edits: Vec<CellAssignment>,
    },
    Insert {
        row_id: String,
        values: Vec<CellAssignment>,
    },
    Delete {
        row_id: String,
        keys: Vec<CellAssignment>,
    },
    /// Insert a full row, resolving collisions on `conflict_columns` (a unique
    /// or primary key) by updating `update_columns` from the proposed values.
    /// An empty `update_columns` compiles to `ON CONFLICT ... DO NOTHING`
    /// (insert-only / skip duplicates). The DB decides per row whether it is an
    /// insert or an update, so there is no read-then-write race. Used by the
    /// CSV import wizard for its insert-only and upsert modes.
    Upsert {
        row_id: String,
        conflict_columns: Vec<String>,
        values: Vec<CellAssignment>,
        update_columns: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct CellAssignment {
    pub column: String,
    pub value: DiffValue,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct DiffValue {
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct TableCommitPreview {
    pub sql: String,
    pub expected_rows: u64,
    pub statement_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct TableCommitResult {
    pub sql: String,
    pub rows_affected: u64,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitPlan {
    pub preview: TableCommitPreview,
    pub statements: Vec<String>,
}

pub fn build_postgres_plan(request: &TableChangeRequest) -> Result<CommitPlan, DiffError> {
    validate_request(request)?;

    let table = qualified_table(&request.schema, &request.table);
    let mut statements = Vec::with_capacity(request.changes.len());
    for change in &request.changes {
        statements.push(statement_for(change, &table)?);
    }

    let mut lines = Vec::with_capacity(statements.len() * 3 + 2);
    lines.push("BEGIN;".to_string());
    for statement in &statements {
        lines.push(String::new());
        lines.push(statement.clone());
    }
    lines.push(String::new());
    lines.push("COMMIT;".to_string());

    Ok(CommitPlan {
        preview: TableCommitPreview {
            sql: lines.join("\n"),
            expected_rows: request.changes.len() as u64,
            statement_count: statements.len() as u32,
        },
        statements,
    })
}

fn validate_request(request: &TableChangeRequest) -> Result<(), DiffError> {
    if request.schema.trim().is_empty() {
        return Err(DiffError::EmptySchema);
    }
    if request.table.trim().is_empty() {
        return Err(DiffError::EmptyTable);
    }
    if request.primary_key.is_empty() && !request.changes.is_empty() {
        return Err(DiffError::MissingPrimaryKey);
    }
    Ok(())
}

fn statement_for(change: &RowChange, table: &str) -> Result<String, DiffError> {
    match change {
        RowChange::Update {
            row_id,
            keys,
            edits,
        } => {
            if keys.is_empty() {
                return Err(DiffError::MissingRowKey(row_id.clone()));
            }
            if edits.is_empty() {
                return Err(DiffError::EmptyEdit(row_id.clone()));
            }
            let set = edits
                .iter()
                .map(|e| format!("  {} = {}", quote_ident(&e.column), literal(&e.value)))
                .collect::<Vec<_>>()
                .join(",\n");
            Ok(format!(
                "UPDATE {table}\nSET\n{set}\nWHERE {};",
                where_clause(keys)
            ))
        }
        RowChange::Insert { row_id, values } => {
            if values.is_empty() {
                return Err(DiffError::EmptyInsert(row_id.clone()));
            }
            let cols = values
                .iter()
                .map(|v| quote_ident(&v.column))
                .collect::<Vec<_>>()
                .join(", ");
            let vals = values
                .iter()
                .map(|v| literal(&v.value))
                .collect::<Vec<_>>()
                .join(", ");
            Ok(format!("INSERT INTO {table} ({cols})\nVALUES ({vals});"))
        }
        RowChange::Delete { row_id, keys } => {
            if keys.is_empty() {
                return Err(DiffError::MissingRowKey(row_id.clone()));
            }
            Ok(format!(
                "DELETE FROM {table}\nWHERE {};",
                where_clause(keys)
            ))
        }
        RowChange::Upsert {
            row_id,
            conflict_columns,
            values,
            update_columns,
        } => {
            if values.is_empty() {
                return Err(DiffError::EmptyInsert(row_id.clone()));
            }
            if conflict_columns.is_empty() {
                return Err(DiffError::MissingConflictTarget(row_id.clone()));
            }
            let cols = values
                .iter()
                .map(|v| quote_ident(&v.column))
                .collect::<Vec<_>>()
                .join(", ");
            let vals = values
                .iter()
                .map(|v| literal(&v.value))
                .collect::<Vec<_>>()
                .join(", ");
            let target = conflict_columns
                .iter()
                .map(|c| quote_ident(c))
                .collect::<Vec<_>>()
                .join(", ");
            // Never overwrite a match-key column, even if the caller listed it.
            let sets = update_columns
                .iter()
                .filter(|c| !conflict_columns.contains(*c))
                .map(|c| format!("  {0} = EXCLUDED.{0}", quote_ident(c)))
                .collect::<Vec<_>>();
            let action = if sets.is_empty() {
                "DO NOTHING".to_string()
            } else {
                format!("DO UPDATE SET\n{}", sets.join(",\n"))
            };
            Ok(format!(
                "INSERT INTO {table} ({cols})\nVALUES ({vals})\nON CONFLICT ({target}) {action};"
            ))
        }
    }
}

fn where_clause(keys: &[CellAssignment]) -> String {
    keys.iter()
        .map(|k| {
            format!(
                "{} IS NOT DISTINCT FROM {}",
                quote_ident(&k.column),
                literal(&k.value)
            )
        })
        .collect::<Vec<_>>()
        .join(" AND ")
}

fn qualified_table(schema: &str, table: &str) -> String {
    format!("{}.{}", quote_ident(schema), quote_ident(table))
}

fn quote_ident(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn literal(value: &DiffValue) -> String {
    match &value.value {
        None => "NULL".to_string(),
        Some(v) => format!("'{}'", v.replace('\'', "''")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_postgres_update_transaction() {
        let plan = build_postgres_plan(&TableChangeRequest {
            database: Some("shop".into()),
            schema: "public".into(),
            table: "orders".into(),
            primary_key: vec!["id".into()],
            columns: vec![],
            changes: vec![RowChange::Update {
                row_id: "1".into(),
                keys: vec![assign("id", Some("1"))],
                edits: vec![assign("status", Some("paid's")), assign("note", None)],
            }],
        })
        .expect("plan");

        assert_eq!(plan.preview.expected_rows, 1);
        assert_eq!(plan.statements.len(), 1);
        assert!(plan.preview.sql.contains("BEGIN;"));
        assert!(plan.preview.sql.contains("\"status\" = 'paid''s'"));
        assert!(plan.preview.sql.contains("\"note\" = NULL"));
        assert!(plan.preview.sql.contains("\"id\" IS NOT DISTINCT FROM '1'"));
    }

    #[test]
    fn rejects_keyless_updates() {
        let err = build_postgres_plan(&TableChangeRequest {
            database: None,
            schema: "public".into(),
            table: "orders".into(),
            primary_key: vec![],
            columns: vec![],
            changes: vec![RowChange::Update {
                row_id: "row:1".into(),
                keys: vec![],
                edits: vec![assign("status", Some("paid"))],
            }],
        })
        .expect_err("keyless update should fail");

        assert_eq!(err, DiffError::MissingPrimaryKey);
    }

    #[test]
    fn builds_upsert_with_on_conflict_do_update() {
        let plan = build_postgres_plan(&TableChangeRequest {
            database: None,
            schema: "public".into(),
            table: "users".into(),
            primary_key: vec!["id".into()],
            columns: vec![],
            changes: vec![RowChange::Upsert {
                row_id: "csv:1".into(),
                conflict_columns: vec!["id".into()],
                values: vec![
                    assign("id", Some("1")),
                    assign("name", Some("Ada")),
                    assign("status", None),
                ],
                // "id" is the match key and must never land in the SET list.
                update_columns: vec!["name".into(), "status".into(), "id".into()],
            }],
        })
        .expect("plan");

        let sql = &plan.preview.sql;
        assert!(sql.contains("INSERT INTO \"public\".\"users\" (\"id\", \"name\", \"status\")"));
        assert!(sql.contains("VALUES ('1', 'Ada', NULL)"));
        assert!(sql.contains("ON CONFLICT (\"id\") DO UPDATE SET"));
        assert!(sql.contains("\"name\" = EXCLUDED.\"name\""));
        assert!(sql.contains("\"status\" = EXCLUDED.\"status\""));
        // match-key column excluded from the SET list
        assert!(!sql.contains("\"id\" = EXCLUDED.\"id\""));
    }

    #[test]
    fn builds_insert_only_upsert_as_do_nothing() {
        let plan = build_postgres_plan(&TableChangeRequest {
            database: None,
            schema: "public".into(),
            table: "users".into(),
            primary_key: vec!["id".into()],
            columns: vec![],
            changes: vec![RowChange::Upsert {
                row_id: "csv:1".into(),
                conflict_columns: vec!["id".into()],
                values: vec![assign("id", Some("1")), assign("name", Some("Ada"))],
                update_columns: vec![],
            }],
        })
        .expect("plan");

        assert!(plan.preview.sql.contains("ON CONFLICT (\"id\") DO NOTHING;"));
    }

    fn assign(column: &str, value: Option<&str>) -> CellAssignment {
        CellAssignment {
            column: column.into(),
            value: DiffValue {
                value: value.map(str::to_string),
            },
        }
    }
}
