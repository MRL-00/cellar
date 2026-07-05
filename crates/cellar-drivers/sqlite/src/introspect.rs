use cellar_core::error::{CellarError, CellarResult};
use cellar_core::schema::{Column, Database, ForeignKey, Index, Schema, Table, View};
use cellar_core::table_browse::mark_primary_keys;
use sqlx::{Row, SqlitePool};

use crate::connect::SqliteConnection;

/// Build the database → schema → table tree for SQLite. One file is one
/// database; everything lives in the built-in `main` schema (ATTACHed
/// databases are out of scope for this slice).
pub async fn introspect(conn: &SqliteConnection) -> CellarResult<Vec<Database>> {
    let pool = conn.pool();

    let objects = sqlx::query(
        "SELECT name, type, sql FROM sqlite_master \
         WHERE type IN ('table', 'view') AND name NOT LIKE 'sqlite_%' \
         ORDER BY name",
    )
    .fetch_all(pool)
    .await
    .map_err(intro_err)?;

    let mut schema = Schema {
        name: "main".to_string(),
        tables: Vec::new(),
        views: Vec::new(),
    };

    for obj in objects {
        let name: String = obj.try_get("name").map_err(intro_err)?;
        let kind: String = obj.try_get("type").map_err(intro_err)?;
        let (columns, pk) = list_columns(pool, &name).await?;

        if kind == "view" {
            let definition: Option<String> = obj.try_get("sql").map_err(intro_err)?;
            schema.views.push(View {
                name,
                schema: "main".to_string(),
                columns,
                definition,
            });
        } else {
            let foreign_keys = list_foreign_keys(pool, &name).await?;
            let indexes = list_indexes(pool, &name, &pk).await?;
            let columns = mark_primary_keys(columns, &pk);
            schema.tables.push(Table {
                name,
                schema: "main".to_string(),
                row_count: None,
                columns,
                primary_key: pk,
                foreign_keys,
                indexes,
            });
        }
    }

    Ok(vec![Database {
        name: "main".to_string(),
        is_default: true,
        schemas: vec![schema],
    }])
}

/// Columns plus the primary key column names in key order, from
/// `pragma_table_info` (`pk` is the 1-based position within the primary key,
/// 0 when the column is not part of it).
async fn list_columns(pool: &SqlitePool, table: &str) -> CellarResult<(Vec<Column>, Vec<String>)> {
    let rows = sqlx::query(
        "SELECT cid, name, type, \"notnull\", dflt_value, pk \
         FROM pragma_table_info(?1) ORDER BY cid",
    )
    .bind(table)
    .fetch_all(pool)
    .await
    .map_err(intro_err)?;

    let mut columns = Vec::with_capacity(rows.len());
    let mut pk_parts: Vec<(i64, String)> = Vec::new();
    for r in rows {
        let cid: i64 = r.try_get("cid").map_err(intro_err)?;
        let name: String = r.try_get("name").map_err(intro_err)?;
        let data_type: String = r.try_get("type").map_err(intro_err)?;
        let notnull: i64 = r.try_get("notnull").map_err(intro_err)?;
        let default: Option<String> = r.try_get("dflt_value").map_err(intro_err)?;
        let pk: i64 = r.try_get("pk").map_err(intro_err)?;
        if pk > 0 {
            pk_parts.push((pk, name.clone()));
        }
        columns.push(Column {
            name,
            data_type: data_type.to_lowercase(),
            nullable: notnull == 0,
            default,
            is_primary_key: false,
            ordinal: (cid + 1) as u32,
            comment: None,
        });
    }
    pk_parts.sort();
    Ok((columns, pk_parts.into_iter().map(|(_, n)| n).collect()))
}

async fn list_foreign_keys(pool: &SqlitePool, table: &str) -> CellarResult<Vec<ForeignKey>> {
    let rows = sqlx::query(
        "SELECT id, seq, \"table\", \"from\", \"to\" \
         FROM pragma_foreign_key_list(?1) ORDER BY id, seq",
    )
    .bind(table)
    .fetch_all(pool)
    .await
    .map_err(intro_err)?;

    let mut out: Vec<ForeignKey> = Vec::new();
    let mut last_id: Option<i64> = None;
    for r in rows {
        let id: i64 = r.try_get("id").map_err(intro_err)?;
        let ref_table: String = r.try_get("table").map_err(intro_err)?;
        let from: String = r.try_get("from").map_err(intro_err)?;
        // `to` is NULL when the FK references the parent's implicit primary
        // key; keep the constraint and skip the unnamed referenced column.
        let to: Option<String> = r.try_get("to").map_err(intro_err)?;

        if last_id != Some(id) {
            // SQLite foreign keys have no names; synthesize a stable one.
            out.push(ForeignKey {
                name: format!("{table}_fk_{id}"),
                columns: Vec::new(),
                referenced_schema: "main".to_string(),
                referenced_table: ref_table,
                referenced_columns: Vec::new(),
            });
            last_id = Some(id);
        }
        let fk = out.last_mut().expect("just pushed");
        fk.columns.push(from);
        if let Some(to) = to {
            fk.referenced_columns.push(to);
        }
    }
    Ok(out)
}

async fn list_indexes(pool: &SqlitePool, table: &str, pk: &[String]) -> CellarResult<Vec<Index>> {
    let rows =
        sqlx::query("SELECT name, \"unique\", origin FROM pragma_index_list(?1) ORDER BY name")
            .bind(table)
            .fetch_all(pool)
            .await
            .map_err(intro_err)?;

    let mut out = Vec::with_capacity(rows.len() + 1);
    for r in rows {
        let name: String = r.try_get("name").map_err(intro_err)?;
        let unique: i64 = r.try_get("unique").map_err(intro_err)?;
        let origin: String = r.try_get("origin").map_err(intro_err)?;

        let col_rows = sqlx::query("SELECT name FROM pragma_index_info(?1) ORDER BY seqno")
            .bind(&name)
            .fetch_all(pool)
            .await
            .map_err(intro_err)?;
        let mut columns = Vec::with_capacity(col_rows.len());
        for c in col_rows {
            // NULL for rowid or expression index parts; keep the index and
            // skip the unnamed part rather than erroring the introspection.
            let col: Option<String> = c.try_get("name").map_err(intro_err)?;
            if let Some(col) = col {
                columns.push(col);
            }
        }

        out.push(Index {
            name,
            columns,
            unique: unique != 0,
            primary: origin == "pk",
        });
    }

    // INTEGER PRIMARY KEY (rowid alias) tables have no pk index in
    // pragma_index_list; synthesize one so the UI still shows the key.
    if !pk.is_empty() && !out.iter().any(|i| i.primary) {
        out.push(Index {
            name: format!("{table}_pk"),
            columns: pk.to_vec(),
            unique: true,
            primary: true,
        });
    }

    Ok(out)
}

fn intro_err(e: sqlx::Error) -> CellarError {
    crate::connect::map_sqlx_err_for_runtime(e, "schema introspection", CellarError::introspection)
}
