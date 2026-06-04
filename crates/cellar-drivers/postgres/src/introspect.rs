use std::collections::BTreeMap;

use cellar_core::error::{CellarError, CellarResult};
use cellar_core::schema::{Column, Database, ForeignKey, Index, Schema, Table, View};
use futures::future::join_all;
use sqlx::{PgPool, Row};

use crate::connect::PgConnection;

/// Build the full database → schema → table tree for every database on the
/// server the user can connect to. Postgres binds a connection to a single
/// database, so each database is introspected through its own pool; the one
/// we're already connected to reuses the live pool. SPEC §6.2.
///
/// Databases the credentials can't open (managed system DBs such as Azure's
/// `azure_sys`, RDS's `rdsadmin`) are still listed, with empty schemas, so the
/// user sees they exist instead of silently dropping them.
pub async fn introspect(conn: &PgConnection) -> CellarResult<Vec<Database>> {
    let pool = conn.pool();
    let current = current_database(pool).await?;
    let db_names = list_databases(pool).await?;

    let tasks = db_names.into_iter().map(|name| {
        let is_default = name == current;
        async move {
            // Reuses the default pool for the connected database and a cached
            // sibling pool for the rest. Databases the credentials can't open
            // are listed empty rather than dropped.
            let schemas = match conn.pool_for_database(&name).await {
                Ok(db_pool) => introspect_schemas(&db_pool).await.unwrap_or_default(),
                Err(_) => Vec::new(),
            };
            Database {
                name,
                is_default,
                schemas,
            }
        }
    });

    Ok(join_all(tasks).await)
}

/// List connectable, non-template databases on the server, ordered by name.
async fn list_databases(pool: &PgPool) -> CellarResult<Vec<String>> {
    let rows = sqlx::query(
        "SELECT datname FROM pg_database \
         WHERE datallowconn AND NOT datistemplate \
         ORDER BY datname",
    )
    .fetch_all(pool)
    .await
    .map_err(intro_err)?;
    rows.into_iter()
        .map(|r| r.try_get::<String, _>("datname").map_err(intro_err))
        .collect()
}

/// Introspect every schema in the database the given pool is bound to.
async fn introspect_schemas(pool: &PgPool) -> CellarResult<Vec<Schema>> {
    let schema_names = list_schemas(pool).await?;
    let columns_by_table = list_columns(pool).await?;
    let primary_keys = list_primary_keys(pool).await?;
    let foreign_keys = list_foreign_keys(pool).await?;
    let indexes = list_indexes(pool).await?;
    let view_defs = list_view_definitions(pool).await?;
    let table_rows = list_tables(pool).await?;

    let mut schemas: BTreeMap<String, Schema> = schema_names
        .into_iter()
        .map(|name| {
            (
                name.clone(),
                Schema {
                    name,
                    tables: Vec::new(),
                    views: Vec::new(),
                },
            )
        })
        .collect();

    for (schema_name, table_name, is_view) in table_rows {
        let key = (schema_name.clone(), table_name.clone());
        let cols = columns_by_table.get(&key).cloned().unwrap_or_default();
        if is_view {
            let definition = view_defs.get(&key).cloned();
            let entry = schemas
                .entry(schema_name.clone())
                .or_insert_with(|| schema_with(schema_name.clone()));
            entry.views.push(View {
                name: table_name,
                schema: schema_name,
                columns: cols,
                definition,
            });
        } else {
            let pk = primary_keys.get(&key).cloned().unwrap_or_default();
            let fks = foreign_keys.get(&key).cloned().unwrap_or_default();
            let idxs = indexes.get(&key).cloned().unwrap_or_default();
            let cols = mark_primary_keys(cols, &pk);
            let entry = schemas
                .entry(schema_name.clone())
                .or_insert_with(|| schema_with(schema_name.clone()));
            entry.tables.push(Table {
                name: table_name,
                schema: schema_name,
                row_count: None,
                columns: cols,
                primary_key: pk,
                foreign_keys: fks,
                indexes: idxs,
            });
        }
    }

    Ok(schemas.into_values().collect())
}

fn schema_with(name: String) -> Schema {
    Schema {
        name,
        tables: Vec::new(),
        views: Vec::new(),
    }
}

fn mark_primary_keys(mut cols: Vec<Column>, pk: &[String]) -> Vec<Column> {
    for c in cols.iter_mut() {
        if pk.iter().any(|p| p == &c.name) {
            c.is_primary_key = true;
        }
    }
    cols
}

async fn current_database(pool: &PgPool) -> CellarResult<String> {
    let row = sqlx::query("SELECT current_database() AS d")
        .fetch_one(pool)
        .await
        .map_err(intro_err)?;
    row.try_get::<String, _>("d").map_err(intro_err)
}

async fn list_schemas(pool: &PgPool) -> CellarResult<Vec<String>> {
    let rows = sqlx::query(
        "SELECT schema_name FROM information_schema.schemata \
         WHERE schema_name NOT IN ('information_schema', 'pg_catalog', 'pg_toast') \
         AND schema_name NOT LIKE 'pg_temp_%' AND schema_name NOT LIKE 'pg_toast_temp_%' \
         ORDER BY schema_name",
    )
    .fetch_all(pool)
    .await
    .map_err(intro_err)?;
    rows.into_iter()
        .map(|r| r.try_get::<String, _>("schema_name").map_err(intro_err))
        .collect()
}

async fn list_tables(pool: &PgPool) -> CellarResult<Vec<(String, String, bool)>> {
    let rows = sqlx::query(
        "SELECT table_schema, table_name, table_type \
         FROM information_schema.tables \
         WHERE table_schema NOT IN ('information_schema', 'pg_catalog') \
         AND table_schema NOT LIKE 'pg_temp_%' AND table_schema NOT LIKE 'pg_toast_temp_%' \
         ORDER BY table_schema, table_name",
    )
    .fetch_all(pool)
    .await
    .map_err(intro_err)?;
    rows.into_iter()
        .map(|r| {
            let schema: String = r.try_get("table_schema").map_err(intro_err)?;
            let name: String = r.try_get("table_name").map_err(intro_err)?;
            let kind: String = r.try_get("table_type").map_err(intro_err)?;
            let is_view = kind == "VIEW" || kind == "MATERIALIZED VIEW";
            Ok((schema, name, is_view))
        })
        .collect()
}

type ColMap = BTreeMap<(String, String), Vec<Column>>;

async fn list_columns(pool: &PgPool) -> CellarResult<ColMap> {
    let rows = sqlx::query(
        "SELECT table_schema, table_name, column_name, \
                udt_name, data_type, is_nullable, column_default, ordinal_position \
         FROM information_schema.columns \
         WHERE table_schema NOT IN ('information_schema', 'pg_catalog') \
         AND table_schema NOT LIKE 'pg_temp_%' AND table_schema NOT LIKE 'pg_toast_temp_%' \
         ORDER BY table_schema, table_name, ordinal_position",
    )
    .fetch_all(pool)
    .await
    .map_err(intro_err)?;

    let mut out: ColMap = BTreeMap::new();
    for r in rows {
        let schema: String = r.try_get("table_schema").map_err(intro_err)?;
        let table: String = r.try_get("table_name").map_err(intro_err)?;
        let column: String = r.try_get("column_name").map_err(intro_err)?;
        let udt: String = r.try_get("udt_name").map_err(intro_err)?;
        let nullable: String = r.try_get("is_nullable").map_err(intro_err)?;
        let default: Option<String> = r.try_get("column_default").map_err(intro_err)?;
        let ordinal: i32 = r.try_get("ordinal_position").map_err(intro_err)?;
        out.entry((schema, table)).or_default().push(Column {
            name: column,
            data_type: udt,
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

async fn list_primary_keys(pool: &PgPool) -> CellarResult<KeyMap> {
    let rows = sqlx::query(
        "SELECT n.nspname AS schema_name, c.relname AS table_name, a.attname AS column_name \
         FROM pg_constraint con \
         JOIN pg_class c ON c.oid = con.conrelid \
         JOIN pg_namespace n ON n.oid = c.relnamespace \
         JOIN unnest(con.conkey) WITH ORDINALITY AS k(attnum, ord) ON true \
         JOIN pg_attribute a ON a.attrelid = c.oid AND a.attnum = k.attnum \
         WHERE con.contype = 'p' \
         AND n.nspname NOT IN ('information_schema', 'pg_catalog') \
         ORDER BY n.nspname, c.relname, k.ord",
    )
    .fetch_all(pool)
    .await
    .map_err(intro_err)?;

    let mut out: KeyMap = BTreeMap::new();
    for r in rows {
        let schema: String = r.try_get("schema_name").map_err(intro_err)?;
        let table: String = r.try_get("table_name").map_err(intro_err)?;
        let col: String = r.try_get("column_name").map_err(intro_err)?;
        out.entry((schema, table)).or_default().push(col);
    }
    Ok(out)
}

type FkMap = BTreeMap<(String, String), Vec<ForeignKey>>;

async fn list_foreign_keys(pool: &PgPool) -> CellarResult<FkMap> {
    // One row per (constraint, position). We aggregate below.
    let rows = sqlx::query(
        "SELECT con.conname AS name, \
                ns.nspname AS schema_name, cl.relname AS table_name, \
                a_local.attname AS local_column, ord AS pos, \
                ref_ns.nspname AS ref_schema, ref_cl.relname AS ref_table, \
                a_ref.attname AS ref_column \
         FROM pg_constraint con \
         JOIN pg_class cl ON cl.oid = con.conrelid \
         JOIN pg_namespace ns ON ns.oid = cl.relnamespace \
         JOIN pg_class ref_cl ON ref_cl.oid = con.confrelid \
         JOIN pg_namespace ref_ns ON ref_ns.oid = ref_cl.relnamespace \
         JOIN unnest(con.conkey) WITH ORDINALITY AS k(attnum, ord) ON true \
         JOIN pg_attribute a_local ON a_local.attrelid = cl.oid AND a_local.attnum = k.attnum \
         JOIN unnest(con.confkey) WITH ORDINALITY AS rk(attnum, ord2) ON ord2 = k.ord \
         JOIN pg_attribute a_ref ON a_ref.attrelid = ref_cl.oid AND a_ref.attnum = rk.attnum \
         WHERE con.contype = 'f' \
         AND ns.nspname NOT IN ('information_schema', 'pg_catalog') \
         ORDER BY ns.nspname, cl.relname, con.conname, ord",
    )
    .fetch_all(pool)
    .await
    .map_err(intro_err)?;

    let mut by_constraint: BTreeMap<(String, String, String), ForeignKey> = BTreeMap::new();
    for r in rows {
        let name: String = r.try_get("name").map_err(intro_err)?;
        let schema: String = r.try_get("schema_name").map_err(intro_err)?;
        let table: String = r.try_get("table_name").map_err(intro_err)?;
        let local: String = r.try_get("local_column").map_err(intro_err)?;
        let ref_schema: String = r.try_get("ref_schema").map_err(intro_err)?;
        let ref_table: String = r.try_get("ref_table").map_err(intro_err)?;
        let ref_col: String = r.try_get("ref_column").map_err(intro_err)?;
        let entry = by_constraint
            .entry((schema.clone(), table.clone(), name.clone()))
            .or_insert_with(|| ForeignKey {
                name,
                columns: Vec::new(),
                referenced_schema: ref_schema,
                referenced_table: ref_table,
                referenced_columns: Vec::new(),
            });
        entry.columns.push(local);
        entry.referenced_columns.push(ref_col);
    }

    let mut out: FkMap = BTreeMap::new();
    for ((schema, table, _), fk) in by_constraint {
        out.entry((schema, table)).or_default().push(fk);
    }
    Ok(out)
}

type IdxMap = BTreeMap<(String, String), Vec<Index>>;

async fn list_indexes(pool: &PgPool) -> CellarResult<IdxMap> {
    let rows = sqlx::query(
        "SELECT ns.nspname AS schema_name, tab.relname AS table_name, \
                idx_cl.relname AS index_name, \
                ix.indisunique AS is_unique, ix.indisprimary AS is_primary, \
                a.attname AS column_name, ord AS col_ord \
         FROM pg_index ix \
         JOIN pg_class idx_cl ON idx_cl.oid = ix.indexrelid \
         JOIN pg_class tab ON tab.oid = ix.indrelid \
         JOIN pg_namespace ns ON ns.oid = tab.relnamespace \
         JOIN unnest(ix.indkey::int[]) WITH ORDINALITY AS k(attnum, ord) ON true \
         JOIN pg_attribute a ON a.attrelid = tab.oid AND a.attnum = k.attnum \
         WHERE ns.nspname NOT IN ('information_schema', 'pg_catalog') \
         AND a.attnum > 0 \
         ORDER BY ns.nspname, tab.relname, idx_cl.relname, ord",
    )
    .fetch_all(pool)
    .await
    .map_err(intro_err)?;

    let mut by_idx: BTreeMap<(String, String, String), Index> = BTreeMap::new();
    for r in rows {
        let schema: String = r.try_get("schema_name").map_err(intro_err)?;
        let table: String = r.try_get("table_name").map_err(intro_err)?;
        let name: String = r.try_get("index_name").map_err(intro_err)?;
        let unique: bool = r.try_get("is_unique").map_err(intro_err)?;
        let primary: bool = r.try_get("is_primary").map_err(intro_err)?;
        let col: String = r.try_get("column_name").map_err(intro_err)?;
        let entry = by_idx
            .entry((schema, table, name.clone()))
            .or_insert_with(|| Index {
                name,
                columns: Vec::new(),
                unique,
                primary,
            });
        entry.columns.push(col);
    }

    let mut out: IdxMap = BTreeMap::new();
    for ((schema, table, _), idx) in by_idx {
        out.entry((schema, table)).or_default().push(idx);
    }
    Ok(out)
}

async fn list_view_definitions(pool: &PgPool) -> CellarResult<BTreeMap<(String, String), String>> {
    let rows = sqlx::query(
        "SELECT table_schema, table_name, view_definition \
         FROM information_schema.views \
         WHERE table_schema NOT IN ('information_schema', 'pg_catalog')",
    )
    .fetch_all(pool)
    .await
    .map_err(intro_err)?;

    let mut out = BTreeMap::new();
    for r in rows {
        let schema: String = r.try_get("table_schema").map_err(intro_err)?;
        let table: String = r.try_get("table_name").map_err(intro_err)?;
        let def: Option<String> = r.try_get("view_definition").map_err(intro_err)?;
        if let Some(d) = def {
            out.insert((schema, table), d);
        }
    }
    Ok(out)
}

fn intro_err(e: sqlx::Error) -> CellarError {
    crate::connect::map_sqlx_err_for_runtime(e, "schema introspection", CellarError::introspection)
}
