use std::collections::BTreeMap;

use cellar_core::error::{CellarError, CellarResult};
use cellar_core::schema::{Column, Database, ForeignKey, Index, Schema, Table, View};
use cellar_core::table_browse::mark_primary_keys;
use sqlx::mysql::MySqlRow;
use sqlx::{MySqlPool, Row};

use crate::connect::MySqlConnection;

/// Build the database → schema → table tree for MySQL.
/// MySQL has a single database (catalog) per connection. We return it as
/// one `Database` entry with a single schema matching the database name.
pub async fn introspect(conn: &MySqlConnection) -> CellarResult<Vec<Database>> {
    let pool = conn.pool();
    let current = current_database(pool).await?;

    let schemas = introspect_schemas(pool, &current).await?;

    Ok(vec![Database {
        name: current,
        is_default: true,
        schemas,
    }])
}

async fn introspect_schemas(pool: &MySqlPool, db_name: &str) -> CellarResult<Vec<Schema>> {
    let columns_by_table = list_columns(pool, db_name).await?;
    let primary_keys = list_primary_keys(pool, db_name).await?;
    let foreign_keys = list_foreign_keys(pool, db_name).await?;
    let indexes = list_indexes(pool, db_name).await?;
    let view_defs = list_view_definitions(pool, db_name).await?;
    let table_rows = list_tables(pool, db_name).await?;

    let mut schema = Schema {
        name: db_name.to_string(),
        tables: Vec::new(),
        views: Vec::new(),
    };

    for (table_name, is_view) in table_rows {
        let key = (db_name.to_string(), table_name.clone());
        let cols = columns_by_table.get(&key).cloned().unwrap_or_default();
        if is_view {
            let definition = view_defs.get(&key).cloned();
            schema.views.push(View {
                name: table_name,
                schema: db_name.to_string(),
                columns: cols,
                definition,
            });
        } else {
            let pk = primary_keys.get(&key).cloned().unwrap_or_default();
            let fks = foreign_keys.get(&key).cloned().unwrap_or_default();
            let idxs = indexes.get(&key).cloned().unwrap_or_default();
            let cols = mark_primary_keys(cols, &pk);
            schema.tables.push(Table {
                name: table_name,
                schema: db_name.to_string(),
                row_count: None,
                columns: cols,
                primary_key: pk,
                foreign_keys: fks,
                indexes: idxs,
            });
        }
    }

    Ok(vec![schema])
}

async fn current_database(pool: &MySqlPool) -> CellarResult<String> {
    let row = sqlx::query("SELECT DATABASE() AS d")
        .fetch_one(pool)
        .await
        .map_err(intro_err)?;
    metadata_text(&row, "d")
}

async fn list_tables(pool: &MySqlPool, db_name: &str) -> CellarResult<Vec<(String, bool)>> {
    let rows = sqlx::query(
        "SELECT TABLE_NAME, TABLE_TYPE \
         FROM information_schema.tables \
         WHERE TABLE_SCHEMA = ? \
         ORDER BY TABLE_NAME",
    )
    .bind(db_name)
    .fetch_all(pool)
    .await
    .map_err(intro_err)?;

    rows.into_iter()
        .map(|r| {
            let name = metadata_text(&r, "TABLE_NAME")?;
            let kind = metadata_text(&r, "TABLE_TYPE")?;
            let is_view = kind == "VIEW";
            Ok((name, is_view))
        })
        .collect()
}

type ColMap = BTreeMap<(String, String), Vec<Column>>;

async fn list_columns(pool: &MySqlPool, db_name: &str) -> CellarResult<ColMap> {
    let rows = sqlx::query(
        "SELECT TABLE_NAME, COLUMN_NAME, DATA_TYPE, COLUMN_TYPE, \
                IS_NULLABLE, COLUMN_DEFAULT, ORDINAL_POSITION \
         FROM information_schema.columns \
         WHERE TABLE_SCHEMA = ? \
         ORDER BY TABLE_NAME, ORDINAL_POSITION",
    )
    .bind(db_name)
    .fetch_all(pool)
    .await
    .map_err(intro_err)?;

    let mut out: ColMap = BTreeMap::new();
    for r in rows {
        let table = metadata_text(&r, "TABLE_NAME")?;
        let column = metadata_text(&r, "COLUMN_NAME")?;
        let data_type = metadata_text(&r, "COLUMN_TYPE")?;
        let nullable = metadata_text(&r, "IS_NULLABLE")?;
        let default = optional_metadata_text(&r, "COLUMN_DEFAULT")?;
        let ordinal: i64 = r.try_get("ORDINAL_POSITION").map_err(intro_err)?;
        out.entry((db_name.to_string(), table))
            .or_default()
            .push(Column {
                name: column,
                data_type,
                nullable: nullable == "YES",
                default,
                is_primary_key: false,
                ordinal: ordinal as u32,
                comment: None,
            });
    }
    Ok(out)
}

type KeyMap = BTreeMap<(String, String), Vec<String>>;

async fn list_primary_keys(pool: &MySqlPool, db_name: &str) -> CellarResult<KeyMap> {
    let rows = sqlx::query(
        "SELECT TABLE_NAME, COLUMN_NAME \
         FROM information_schema.key_column_usage \
         WHERE TABLE_SCHEMA = ? AND CONSTRAINT_NAME = 'PRIMARY' \
         ORDER BY TABLE_NAME, ORDINAL_POSITION",
    )
    .bind(db_name)
    .fetch_all(pool)
    .await
    .map_err(intro_err)?;

    let mut out: KeyMap = BTreeMap::new();
    for r in rows {
        let table = metadata_text(&r, "TABLE_NAME")?;
        let col = metadata_text(&r, "COLUMN_NAME")?;
        out.entry((db_name.to_string(), table))
            .or_default()
            .push(col);
    }
    Ok(out)
}

type FkMap = BTreeMap<(String, String), Vec<ForeignKey>>;

async fn list_foreign_keys(pool: &MySqlPool, db_name: &str) -> CellarResult<FkMap> {
    let rows = sqlx::query(
        "SELECT TABLE_NAME, CONSTRAINT_NAME, COLUMN_NAME, \
                REFERENCED_TABLE_NAME, REFERENCED_COLUMN_NAME \
         FROM information_schema.key_column_usage \
         WHERE TABLE_SCHEMA = ? AND REFERENCED_TABLE_NAME IS NOT NULL \
         ORDER BY TABLE_NAME, CONSTRAINT_NAME, ORDINAL_POSITION",
    )
    .bind(db_name)
    .fetch_all(pool)
    .await
    .map_err(intro_err)?;

    let mut by_constraint: BTreeMap<(String, String), ForeignKey> = BTreeMap::new();
    for r in rows {
        let table = metadata_text(&r, "TABLE_NAME")?;
        let name = metadata_text(&r, "CONSTRAINT_NAME")?;
        let local = metadata_text(&r, "COLUMN_NAME")?;
        let ref_table = metadata_text(&r, "REFERENCED_TABLE_NAME")?;
        let ref_col = metadata_text(&r, "REFERENCED_COLUMN_NAME")?;

        let entry = by_constraint
            .entry((table.clone(), name.clone()))
            .or_insert_with(|| ForeignKey {
                name,
                columns: Vec::new(),
                referenced_schema: db_name.to_string(),
                referenced_table: ref_table,
                referenced_columns: Vec::new(),
            });
        entry.columns.push(local);
        entry.referenced_columns.push(ref_col);
    }

    let mut out: FkMap = BTreeMap::new();
    for ((table, _), fk) in by_constraint {
        out.entry((db_name.to_string(), table))
            .or_default()
            .push(fk);
    }
    Ok(out)
}

type IdxMap = BTreeMap<(String, String), Vec<Index>>;

async fn list_indexes(pool: &MySqlPool, db_name: &str) -> CellarResult<IdxMap> {
    let rows = sqlx::query(
        "SELECT TABLE_NAME, INDEX_NAME, COLUMN_NAME, \
                NON_UNIQUE, SEQ_IN_INDEX \
         FROM information_schema.statistics \
         WHERE TABLE_SCHEMA = ? \
         ORDER BY TABLE_NAME, INDEX_NAME, SEQ_IN_INDEX",
    )
    .bind(db_name)
    .fetch_all(pool)
    .await
    .map_err(intro_err)?;

    let mut by_idx: BTreeMap<(String, String, bool), Index> = BTreeMap::new();
    for r in rows {
        let table = metadata_text(&r, "TABLE_NAME")?;
        let name = metadata_text(&r, "INDEX_NAME")?;
        // COLUMN_NAME is NULL for MySQL 8+ expression (functional) index parts;
        // the expression lives in a separate EXPRESSION column we don't model.
        // Keep the index but skip the unnamed part rather than erroring the
        // whole introspection.
        let col = optional_metadata_text(&r, "COLUMN_NAME")?;
        let non_unique: i64 = r.try_get("NON_UNIQUE").map_err(intro_err)?;
        let unique = non_unique == 0;
        let primary = name == "PRIMARY";
        let entry = by_idx
            .entry((table, name.clone(), primary))
            .or_insert_with(|| Index {
                name,
                columns: Vec::new(),
                unique,
                primary,
            });
        if let Some(col) = col {
            entry.columns.push(col);
        }
    }

    let mut out: IdxMap = BTreeMap::new();
    for ((table, _, _), idx) in by_idx {
        out.entry((db_name.to_string(), table))
            .or_default()
            .push(idx);
    }
    Ok(out)
}

async fn list_view_definitions(
    pool: &MySqlPool,
    db_name: &str,
) -> CellarResult<BTreeMap<(String, String), String>> {
    let rows = sqlx::query(
        "SELECT TABLE_NAME, VIEW_DEFINITION \
         FROM information_schema.views \
         WHERE TABLE_SCHEMA = ?",
    )
    .bind(db_name)
    .fetch_all(pool)
    .await
    .map_err(intro_err)?;

    let mut out = BTreeMap::new();
    for r in rows {
        let table = metadata_text(&r, "TABLE_NAME")?;
        let def = optional_metadata_text(&r, "VIEW_DEFINITION")?;
        if let Some(d) = def {
            out.insert((db_name.to_string(), table), d);
        }
    }
    Ok(out)
}

// Aurora/MySQL can report textual catalog fields with a binary collation,
// which SQLx classifies as VARBINARY. These specific metadata fields are known
// to contain text: bypass SQL type compatibility, but retain SQLx's strict
// UTF-8 decoding, NULL handling, and column-specific errors. Do not use this
// for arbitrary query results, where binary values must remain binary.
fn metadata_text(row: &MySqlRow, column: &str) -> CellarResult<String> {
    row.try_get_unchecked::<String, _>(column)
        .map_err(intro_err)
}

fn optional_metadata_text(row: &MySqlRow, column: &str) -> CellarResult<Option<String>> {
    row.try_get_unchecked::<Option<String>, _>(column)
        .map_err(intro_err)
}

#[cfg(all(test, feature = "integration-tests"))]
#[path = "introspect_tests.rs"]
mod tests;

fn intro_err(e: sqlx::Error) -> CellarError {
    crate::connect::map_sqlx_err_for_runtime(e, "schema introspection", CellarError::introspection)
}
