//! Build reviewable, transactional SQL from grid pending changes.

use cellar_sql::Dialect;
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
    build_plan(request, Dialect::Postgres)
}

/// T-SQL plan for SQL Server / Azure SQL: bracket quoting, null-safe `=` in
/// key predicates (portable to pre-2022 servers without
/// `IS NOT DISTINCT FROM`), and `MERGE` for upserts.
pub fn build_mssql_plan(request: &TableChangeRequest) -> Result<CommitPlan, DiffError> {
    build_plan(request, Dialect::Mssql)
}

fn build_plan(request: &TableChangeRequest, dialect: Dialect) -> Result<CommitPlan, DiffError> {
    validate_request(request)?;

    let table = dialect.quote_qualified(&request.schema, &request.table);
    let mut statements = Vec::with_capacity(request.changes.len());
    for change in &request.changes {
        statements.push(statement_for(change, &table, dialect)?);
    }

    let (begin, commit) = match dialect {
        Dialect::Mssql => ("BEGIN TRANSACTION;", "COMMIT TRANSACTION;"),
        _ => ("BEGIN;", "COMMIT;"),
    };
    let mut lines = Vec::with_capacity(statements.len() * 3 + 2);
    lines.push(begin.to_string());
    for statement in &statements {
        lines.push(String::new());
        lines.push(statement.clone());
    }
    lines.push(String::new());
    lines.push(commit.to_string());

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
    if request.primary_key.is_empty()
        && request
            .changes
            .iter()
            .any(|change| matches!(change, RowChange::Update { .. } | RowChange::Delete { .. }))
    {
        return Err(DiffError::MissingPrimaryKey);
    }
    Ok(())
}

fn statement_for(change: &RowChange, table: &str, dialect: Dialect) -> Result<String, DiffError> {
    let quote_ident = |ident: &str| dialect.quote_ident(ident);
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
                where_clause(keys, dialect)
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
                where_clause(keys, dialect)
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
            if matches!(dialect, Dialect::Mssql) {
                return merge_statement(table, conflict_columns, values, update_columns, dialect);
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

/// T-SQL upsert. SQL Server has no `ON CONFLICT`, so an upsert compiles to a
/// single-row `MERGE`: the proposed values form the source rowset, matching on
/// the conflict columns. An empty update set omits `WHEN MATCHED` (insert-only,
/// skip duplicates) — the T-SQL equivalent of `DO NOTHING`.
fn merge_statement(
    table: &str,
    conflict_columns: &[String],
    values: &[CellAssignment],
    update_columns: &[String],
    dialect: Dialect,
) -> Result<String, DiffError> {
    let quote_ident = |ident: &str| dialect.quote_ident(ident);
    let source = values
        .iter()
        .map(|v| format!("{} AS {}", literal(&v.value), quote_ident(&v.column)))
        .collect::<Vec<_>>()
        .join(", ");
    let on = conflict_columns
        .iter()
        .map(|c| {
            let c = quote_ident(c);
            // Null-safe match, same semantics as ON CONFLICT on a nullable key.
            format!("(t.{c} = s.{c} OR (t.{c} IS NULL AND s.{c} IS NULL))")
        })
        .collect::<Vec<_>>()
        .join(" AND ");
    // Never overwrite a match-key column, even if the caller listed it.
    let sets = update_columns
        .iter()
        .filter(|c| !conflict_columns.contains(*c))
        .map(|c| format!("  {0} = s.{0}", quote_ident(c)))
        .collect::<Vec<_>>();
    let matched = if sets.is_empty() {
        String::new()
    } else {
        format!("WHEN MATCHED THEN UPDATE SET\n{}\n", sets.join(",\n"))
    };
    let cols = values
        .iter()
        .map(|v| quote_ident(&v.column))
        .collect::<Vec<_>>()
        .join(", ");
    let src_vals = values
        .iter()
        .map(|v| format!("s.{}", quote_ident(&v.column)))
        .collect::<Vec<_>>()
        .join(", ");
    Ok(format!(
        "MERGE {table} AS t\nUSING (SELECT {source}) AS s\nON {on}\n{matched}WHEN NOT MATCHED THEN INSERT ({cols})\nVALUES ({src_vals});"
    ))
}

fn where_clause(keys: &[CellAssignment], dialect: Dialect) -> String {
    keys.iter()
        .map(|k| {
            let ident = dialect.quote_ident(&k.column);
            match dialect {
                // T-SQL before 2022 lacks IS NOT DISTINCT FROM; the literal is
                // known here, so NULL keys compile straight to IS NULL.
                Dialect::Mssql => match &k.value.value {
                    None => format!("{ident} IS NULL"),
                    Some(_) => format!("{ident} = {}", literal(&k.value)),
                },
                _ => format!("{ident} IS NOT DISTINCT FROM {}", literal(&k.value)),
            }
        })
        .collect::<Vec<_>>()
        .join(" AND ")
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

        assert!(plan
            .preview
            .sql
            .contains("ON CONFLICT (\"id\") DO NOTHING;"));
    }

    #[test]
    fn builds_mssql_update_with_null_safe_keys() {
        let plan = build_mssql_plan(&TableChangeRequest {
            database: Some("epiczone".into()),
            schema: "epiczone".into(),
            table: "Settings".into(),
            primary_key: vec!["Id".into()],
            columns: vec![],
            changes: vec![RowChange::Update {
                row_id: "1".into(),
                keys: vec![assign("Id", Some("c532")), assign("Region", None)],
                edits: vec![assign("Enabled", Some("true"))],
            }],
        })
        .expect("plan");

        let sql = &plan.preview.sql;
        assert!(sql.contains("BEGIN TRANSACTION;"));
        assert!(sql.contains("UPDATE [epiczone].[Settings]"));
        assert!(sql.contains("[Enabled] = 'true'"));
        assert!(sql.contains("[Id] = 'c532'"));
        assert!(sql.contains("[Region] IS NULL"));
        assert!(!sql.contains("IS NOT DISTINCT FROM"));
        assert!(sql.contains("COMMIT TRANSACTION;"));
    }

    #[test]
    fn builds_mssql_upsert_as_merge() {
        let plan = build_mssql_plan(&TableChangeRequest {
            database: None,
            schema: "dbo".into(),
            table: "users".into(),
            primary_key: vec!["id".into()],
            columns: vec![],
            changes: vec![RowChange::Upsert {
                row_id: "csv:1".into(),
                conflict_columns: vec!["id".into()],
                values: vec![assign("id", Some("1")), assign("name", Some("Ada"))],
                update_columns: vec!["name".into(), "id".into()],
            }],
        })
        .expect("plan");

        let sql = &plan.preview.sql;
        assert!(sql.contains("MERGE [dbo].[users] AS t"));
        assert!(sql.contains("USING (SELECT '1' AS [id], 'Ada' AS [name]) AS s"));
        assert!(sql.contains("(t.[id] = s.[id] OR (t.[id] IS NULL AND s.[id] IS NULL))"));
        assert!(sql.contains("WHEN MATCHED THEN UPDATE SET\n  [name] = s.[name]"));
        // match-key column excluded from the SET list
        assert!(!sql.contains("[id] = s.[id]\n"));
        assert!(sql.contains("WHEN NOT MATCHED THEN INSERT ([id], [name])"));
        assert!(sql.contains("VALUES (s.[id], s.[name]);"));
    }

    #[test]
    fn builds_mssql_insert_only_upsert_without_when_matched() {
        let plan = build_mssql_plan(&TableChangeRequest {
            database: None,
            schema: "dbo".into(),
            table: "users".into(),
            primary_key: vec!["id".into()],
            columns: vec![],
            changes: vec![RowChange::Upsert {
                row_id: "csv:1".into(),
                conflict_columns: vec!["id".into()],
                values: vec![assign("id", Some("1"))],
                update_columns: vec![],
            }],
        })
        .expect("plan");

        assert!(!plan.preview.sql.contains("WHEN MATCHED"));
        assert!(plan.preview.sql.contains("WHEN NOT MATCHED THEN INSERT"));
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
