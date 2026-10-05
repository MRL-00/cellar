use crate::service::{self, Request};
use cellar_diff::{CellAssignment, DiffColumn, DiffValue, RowChange, TableChangeRequest};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{Connection, Postgres, Row, SqliteConnection, Transaction};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Change {
    pub kind: String,
    pub original: Option<Vec<Value>>,
    pub revision: Option<String>,
    pub values: std::collections::BTreeMap<String, Option<String>>,
}

pub fn revision(cells: &[Value]) -> String {
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(cells).unwrap_or_default())
    )
}

struct Plan {
    statements: Vec<String>,
    predicates: Vec<Option<String>>,
    columns: Vec<String>,
    sql: String,
    id: String,
}

fn plan(
    request: &Request,
    metadata: Vec<(String, String, bool, bool)>,
    postgres: bool,
) -> Result<Plan, String> {
    let changes = request.changes.as_ref().ok_or("No pending changes")?;
    if changes.is_empty() || changes.len() > 50 {
        return Err("Review between 1 and 50 changes at a time".into());
    }
    let columns = metadata.iter().map(|c| c.0.clone()).collect::<Vec<_>>();
    let primary = metadata
        .iter()
        .filter(|c| c.3)
        .map(|c| c.0.clone())
        .collect::<Vec<_>>();
    let schema = request.schema.clone().ok_or("Select a schema")?;
    let table = request.table.clone().ok_or("Select a table")?;
    let mut rows = Vec::new();
    let mut predicates = Vec::new();
    for (index, change) in changes.iter().enumerate() {
        if change.values.len() > 128
            || change.values.iter().any(|(name, value)| {
                !columns.contains(name) || value.as_ref().is_some_and(|v| v.len() > 16384)
            })
        {
            return Err("Invalid changed column or oversized value".into());
        }
        let assignments = change
            .values
            .iter()
            .map(|(column, value)| CellAssignment {
                column: column.clone(),
                value: DiffValue {
                    value: value.clone(),
                },
            })
            .collect::<Vec<_>>();
        let row_id = index.to_string();
        if change.kind == "insert" {
            rows.push(RowChange::Insert {
                row_id,
                values: assignments,
            });
            predicates.push(None);
            continue;
        }
        let original = change.original.as_ref().ok_or("Original row is required")?;
        if primary.is_empty() || original.len() != columns.len() || change.revision.is_none() {
            return Err(
                "Editing and deleting require a primary key and complete original row".into(),
            );
        }
        if original.iter().any(|v| {
            v.as_str().is_some_and(|s| {
                s.contains("[cell truncated]") || s == "[binary or unsupported value]"
            })
        }) {
            return Err("Truncated or binary rows cannot be edited in this workspace".into());
        }
        let keys = primary
            .iter()
            .map(|name| {
                let value = &original[columns.iter().position(|c| c == name).unwrap()];
                CellAssignment {
                    column: name.clone(),
                    value: DiffValue {
                        value: match value {
                            Value::Null => None,
                            Value::String(s) => Some(s.clone()),
                            other => Some(other.to_string()),
                        },
                    },
                }
            })
            .collect::<Vec<_>>();
        let predicate = keys
            .iter()
            .map(|key| {
                format!(
                    "{} IS NOT DISTINCT FROM {}",
                    cellar_sql::Dialect::Postgres.quote_ident(&key.column),
                    key.value
                        .value
                        .as_ref()
                        .map(|v| cellar_sql::quote_string_literal(v))
                        .unwrap_or("NULL".into())
                )
            })
            .collect::<Vec<_>>()
            .join(" AND ");
        predicates.push(Some(predicate));
        rows.push(match change.kind.as_str() {
            "update" => RowChange::Update {
                row_id,
                keys,
                edits: assignments,
            },
            "delete" => RowChange::Delete { row_id, keys },
            _ => return Err("Unsupported row change".into()),
        });
    }
    let result = cellar_diff::build_postgres_plan(&TableChangeRequest {
        database: None,
        schema,
        table,
        primary_key: primary,
        columns: metadata
            .into_iter()
            .map(|c| DiffColumn {
                name: c.0,
                data_type: c.1,
                nullable: c.2,
            })
            .collect(),
        changes: rows,
    })
    .map_err(|_| "Cannot build transactional change plan")?;
    let id = format!("{:x}",Sha256::digest(serde_json::to_vec(&json!({"connection":request.connection,"sql":result.preview.sql,"changes":changes,"postgres":postgres})).unwrap()));
    if request.action == "commit"
        && (request.preview_id.as_ref() != Some(&id)
            || request.confirmation.as_deref()
                != Some(&format!("COMMIT {} CHANGES", changes.len())))
    {
        return Err("Review the current SQL plan and explicitly confirm before committing".into());
    }
    Ok(Plan {
        statements: result.statements,
        predicates,
        columns,
        sql: result.preview.sql,
        id,
    })
}

fn preview(plan: &Plan) -> Value {
    json!({"sql":plan.sql,"previewId":plan.id,"changeCount":plan.statements.len()})
}

pub async fn sqlite(
    connection: &mut SqliteConnection,
    request: &Request,
    writing: bool,
) -> Result<Value, String> {
    if request.schema.as_deref() != Some("main") {
        return Err("Unknown SQLite schema".into());
    }
    let table = request.table.as_deref().ok_or("Select a table")?;
    let kind: Option<String> = sqlx::query_scalar(
        "SELECT type FROM sqlite_schema WHERE name=? AND name NOT LIKE 'sqlite_%'",
    )
    .bind(table)
    .fetch_optional(&mut *connection)
    .await
    .map_err(|_| "Table unavailable")?;
    if kind.as_deref() != Some("table") {
        return Err("Views and system tables are read-only".into());
    }
    let metadata = sqlx::query(
        "SELECT name,type,\"notnull\",pk FROM pragma_table_info(?) ORDER BY cid LIMIT 129",
    )
    .bind(table)
    .fetch_all(&mut *connection)
    .await
    .map_err(|_| "Table metadata unavailable")?;
    if metadata.len() > 128 {
        return Err("Table exceeds column limit".into());
    }
    let plan = plan(
        request,
        metadata
            .iter()
            .map(|r| {
                (
                    r.get(0),
                    r.get(1),
                    r.get::<i64, _>(2) == 0,
                    r.get::<i64, _>(3) > 0,
                )
            })
            .collect(),
        false,
    )?;
    if !writing {
        return Ok(preview(&plan));
    }
    let mut transaction = connection
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|_| "Cannot acquire write transaction")?;
    for (index, statement) in plan.statements.iter().enumerate() {
        if let Some(predicate) = &plan.predicates[index] {
            let sql = format!(
                "SELECT {} FROM {} WHERE {predicate} LIMIT 2",
                plan.columns
                    .iter()
                    .map(|c| cellar_sql::Dialect::Sqlite.quote_ident(c))
                    .collect::<Vec<_>>()
                    .join(","),
                cellar_sql::Dialect::Sqlite.quote_qualified("main", table)
            );
            let rows = sqlx::query(&sql)
                .fetch_all(&mut *transaction)
                .await
                .map_err(|_| "Cannot validate original row")?;
            if rows.len() != 1
                || request.changes.as_ref().unwrap()[index].revision.as_ref()
                    != Some(&revision(&service::sqlite_cells(&rows[0])))
            {
                return Err("Row changed since it was loaded; transaction rolled back. Refresh and review again.".into());
            }
        }
        let affected = sqlx::query(statement)
            .execute(&mut *transaction)
            .await
            .map_err(|_| "Change violates database constraints; transaction rolled back")?
            .rows_affected();
        if affected != 1 {
            return Err(
                "Change affected an unexpected number of rows; transaction rolled back".into(),
            );
        }
    }
    transaction.commit().await.map_err(|_| "Commit failed")?;
    Ok(json!({"rowsAffected":plan.statements.len(),"committed":true}))
}

pub async fn postgres(
    mut transaction: Transaction<'_, Postgres>,
    request: &Request,
    writing: bool,
) -> Result<Value, String> {
    let schema = request.schema.as_deref().ok_or("Select a schema")?;
    let table = request.table.as_deref().ok_or("Select a table")?;
    let metadata=sqlx::query("SELECT a.attname,format_type(a.atttypid,a.atttypmod),NOT a.attnotnull,EXISTS(SELECT 1 FROM pg_index i WHERE i.indrelid=c.oid AND i.indisprimary AND a.attnum=ANY(i.indkey)) FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace JOIN pg_attribute a ON a.attrelid=c.oid WHERE n.nspname=$1 AND c.relname=$2 AND c.relkind='r' AND a.attnum>0 AND NOT a.attisdropped ORDER BY a.attnum LIMIT 129").bind(schema).bind(table).fetch_all(&mut *transaction).await.map_err(|_|"Table metadata unavailable")?;
    if metadata.is_empty() || metadata.len() > 128 {
        return Err("Only ordinary tables with bounded columns can be edited".into());
    }
    let plan = plan(
        request,
        metadata
            .iter()
            .map(|r| (r.get(0), r.get(1), r.get(2), r.get(3)))
            .collect(),
        true,
    )?;
    if !writing {
        transaction
            .rollback()
            .await
            .map_err(|_| "Read transaction cleanup failed")?;
        return Ok(preview(&plan));
    }
    for (index, statement) in plan.statements.iter().enumerate() {
        if let Some(predicate) = &plan.predicates[index] {
            let sql=format!("SELECT to_jsonb(cellar_row)::text FROM {} AS cellar_row WHERE {predicate} FOR UPDATE",cellar_sql::Dialect::Postgres.quote_qualified(schema,table));
            let rows = sqlx::query(&sql)
                .fetch_all(&mut *transaction)
                .await
                .map_err(|_| "Cannot lock original row")?;
            if rows.len() != 1 {
                return Err("Row no longer exists; transaction rolled back".into());
            }
            let object: Value = serde_json::from_str(&rows[0].get::<String, _>(0))
                .map_err(|_| "Cannot decode original row")?;
            if request.changes.as_ref().unwrap()[index].revision.as_ref()
                != Some(&revision(&service::postgres_cells(&object, &plan.columns)))
            {
                return Err("Row changed since it was loaded; transaction rolled back. Refresh and review again.".into());
            }
        }
        let affected = sqlx::query(statement)
            .execute(&mut *transaction)
            .await
            .map_err(|_| "Change violates database constraints; transaction rolled back")?
            .rows_affected();
        if affected != 1 {
            return Err(
                "Change affected an unexpected number of rows; transaction rolled back".into(),
            );
        }
    }
    transaction.commit().await.map_err(|_| "Commit failed")?;
    Ok(json!({"rowsAffected":plan.statements.len(),"committed":true}))
}
