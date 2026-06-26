//! "Find Usages" catalog reads and reference search for Postgres.
//!
//! Two halves, kept apart so the host can cache between them:
//!
//! - [`fetch_usage_definitions`] pulls the text of every view, materialized
//!   view, function, procedure, trigger, and constraint in a database from the
//!   system catalogs. This is the expensive part (many catalog round-trips and
//!   `pg_get_*def` calls), so the host caches the result per connection+database
//!   and invalidates it on schema refresh.
//! - [`search_usages`] runs over those cached definitions and asks `cellar-sql`
//!   to *structurally* confirm each reference, so a substring like
//!   `user_identities` never matches a search for `user`.

use cellar_core::error::{CellarError, CellarResult};
use cellar_core::schema::{UsageDefinition, UsageKind, UsageReference};
use sqlx::{PgPool, Row};

use crate::connect::PgConnection;

/// Schemas we never look inside — system catalogs and transient temp schemas.
const SYSTEM_SCHEMA_FILTER: &str = "n.nspname NOT IN ('information_schema', 'pg_catalog') \
     AND n.nspname NOT LIKE 'pg_temp_%' AND n.nspname NOT LIKE 'pg_toast_temp_%'";

/// Read every searchable object definition in `database` from the catalogs.
pub async fn fetch_usage_definitions(
    pg: &PgConnection,
    database: &str,
) -> CellarResult<Vec<UsageDefinition>> {
    let pool = pg.pool_for_database(database).await?;
    let mut defs = Vec::new();
    defs.extend(fetch_views(&pool).await?);
    defs.extend(fetch_routines(&pool).await?);
    defs.extend(fetch_triggers(&pool).await?);
    defs.extend(fetch_constraints(&pool).await?);
    Ok(defs)
}

/// Structurally confirm references to `target_schema`.`object` (optionally
/// narrowed to `column`) across the cached definitions. `schema_filter`, when
/// set, limits results to referencing objects in that schema (the default
/// "current schema" scope); `None` searches every schema in the database.
/// `target_schema` is the schema of the searched table — it disambiguates
/// schema-qualified references so `other.users` isn't a usage of `public.users`.
pub fn search_usages(
    defs: &[UsageDefinition],
    schema_filter: Option<&str>,
    target_schema: &str,
    object: &str,
    column: Option<&str>,
) -> Vec<UsageReference> {
    let mut out: Vec<UsageReference> = Vec::new();
    for def in defs {
        if let Some(schema) = schema_filter {
            if def.schema != schema {
                continue;
            }
        }
        let refs = cellar_sql::find_references(&def.definition, target_schema, object, column);
        // One result per object, anchored at its first confirmed reference, so
        // the panel stays readable for definitions that mention the name often.
        if let Some(first) = refs.first() {
            out.push(UsageReference {
                kind: def.kind,
                schema: def.schema.clone(),
                name: def.name.clone(),
                on_table: def.on_table.clone(),
                line: first.line,
                snippet: first.snippet.clone(),
                matched_column: first.matched_column.clone(),
                definition: def.definition.clone(),
            });
        }
    }
    out.sort_by(|a, b| {
        (a.schema.as_str(), a.name.as_str()).cmp(&(b.schema.as_str(), b.name.as_str()))
    });
    out
}

async fn fetch_views(pool: &PgPool) -> CellarResult<Vec<UsageDefinition>> {
    // Regular views and materialized views in one pass. `pg_get_viewdef`
    // pretty-prints with newlines so line numbers in the result are meaningful.
    let rows = sqlx::query(&format!(
        "SELECT n.nspname AS schema_name, c.relname AS name, \
                c.relkind::text AS kind, \
                pg_catalog.pg_get_viewdef(c.oid, true) AS def \
         FROM pg_catalog.pg_class c \
         JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace \
         WHERE c.relkind IN ('v', 'm') AND {SYSTEM_SCHEMA_FILTER} \
         ORDER BY n.nspname, c.relname"
    ))
    .fetch_all(pool)
    .await
    .map_err(usage_err)?;

    let mut out = Vec::new();
    for r in rows {
        let schema: String = r.try_get("schema_name").map_err(usage_err)?;
        let name: String = r.try_get("name").map_err(usage_err)?;
        let kind: String = r.try_get("kind").map_err(usage_err)?;
        let def: Option<String> = r.try_get("def").map_err(usage_err)?;
        let Some(def) = def else { continue };
        out.push(UsageDefinition {
            kind: if kind == "m" {
                UsageKind::MaterializedView
            } else {
                UsageKind::View
            },
            schema,
            name,
            on_table: None,
            definition: def,
        });
    }
    Ok(out)
}

async fn fetch_routines(pool: &PgPool) -> CellarResult<Vec<UsageDefinition>> {
    // `pg_get_functiondef` gives the full CREATE statement (signature + body),
    // which is both what we parse and what the UI opens. It can raise for exotic
    // routines, so on any failure we fall back to the raw `prosrc` body, which
    // never errors. `prokind` 'f' = function, 'p' = procedure.
    let full = format!(
        "SELECT n.nspname AS schema_name, p.proname AS name, \
                p.prokind::text AS kind, pg_catalog.pg_get_functiondef(p.oid) AS def \
         FROM pg_catalog.pg_proc p \
         JOIN pg_catalog.pg_namespace n ON n.oid = p.pronamespace \
         WHERE p.prokind IN ('f', 'p') AND {SYSTEM_SCHEMA_FILTER} \
         ORDER BY n.nspname, p.proname"
    );
    let fallback = format!(
        "SELECT n.nspname AS schema_name, p.proname AS name, \
                p.prokind::text AS kind, p.prosrc AS def \
         FROM pg_catalog.pg_proc p \
         JOIN pg_catalog.pg_namespace n ON n.oid = p.pronamespace \
         WHERE p.prokind IN ('f', 'p') AND {SYSTEM_SCHEMA_FILTER} \
         ORDER BY n.nspname, p.proname"
    );

    let rows = match sqlx::query(&full).fetch_all(pool).await {
        Ok(rows) => rows,
        Err(_) => sqlx::query(&fallback)
            .fetch_all(pool)
            .await
            .map_err(usage_err)?,
    };

    let mut out = Vec::new();
    for r in rows {
        let schema: String = r.try_get("schema_name").map_err(usage_err)?;
        let name: String = r.try_get("name").map_err(usage_err)?;
        let kind: String = r.try_get("kind").map_err(usage_err)?;
        let def: Option<String> = r.try_get("def").map_err(usage_err)?;
        let Some(def) = def else { continue };
        out.push(UsageDefinition {
            kind: if kind == "p" {
                UsageKind::Procedure
            } else {
                UsageKind::Function
            },
            schema,
            name,
            on_table: None,
            definition: def,
        });
    }
    Ok(out)
}

async fn fetch_triggers(pool: &PgPool) -> CellarResult<Vec<UsageDefinition>> {
    // User triggers only (`tgisinternal` excludes FK-enforcement triggers).
    // `pg_get_triggerdef` includes the WHEN clause and the called function, so
    // the table/column the trigger watches shows up structurally.
    let rows = sqlx::query(&format!(
        "SELECT n.nspname AS schema_name, c.relname AS table_name, \
                t.tgname AS name, pg_catalog.pg_get_triggerdef(t.oid) AS def \
         FROM pg_catalog.pg_trigger t \
         JOIN pg_catalog.pg_class c ON c.oid = t.tgrelid \
         JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace \
         WHERE NOT t.tgisinternal AND {SYSTEM_SCHEMA_FILTER} \
         ORDER BY n.nspname, c.relname, t.tgname"
    ))
    .fetch_all(pool)
    .await
    .map_err(usage_err)?;

    let mut out = Vec::new();
    for r in rows {
        let schema: String = r.try_get("schema_name").map_err(usage_err)?;
        let table: String = r.try_get("table_name").map_err(usage_err)?;
        let name: String = r.try_get("name").map_err(usage_err)?;
        let def: Option<String> = r.try_get("def").map_err(usage_err)?;
        let Some(def) = def else { continue };
        out.push(UsageDefinition {
            kind: UsageKind::Trigger,
            schema,
            name,
            on_table: Some(table),
            definition: def,
        });
    }
    Ok(out)
}

async fn fetch_constraints(pool: &PgPool) -> CellarResult<Vec<UsageDefinition>> {
    // Foreign-key and check constraints. `pg_get_constraintdef` yields text like
    // `FOREIGN KEY (user_id) REFERENCES users(id)`, so a FK that *references* the
    // searched table is found via the `REFERENCES <table>` token, and check
    // constraints are found via the columns they mention.
    let rows = sqlx::query(&format!(
        "SELECT n.nspname AS schema_name, c.relname AS table_name, \
                con.conname AS name, pg_catalog.pg_get_constraintdef(con.oid) AS def \
         FROM pg_catalog.pg_constraint con \
         JOIN pg_catalog.pg_class c ON c.oid = con.conrelid \
         JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace \
         WHERE con.contype IN ('f', 'c') AND {SYSTEM_SCHEMA_FILTER} \
         ORDER BY n.nspname, c.relname, con.conname"
    ))
    .fetch_all(pool)
    .await
    .map_err(usage_err)?;

    let mut out = Vec::new();
    for r in rows {
        let schema: String = r.try_get("schema_name").map_err(usage_err)?;
        let table: String = r.try_get("table_name").map_err(usage_err)?;
        let name: String = r.try_get("name").map_err(usage_err)?;
        let def: Option<String> = r.try_get("def").map_err(usage_err)?;
        let Some(def) = def else { continue };
        out.push(UsageDefinition {
            kind: UsageKind::Constraint,
            schema,
            name,
            on_table: Some(table),
            definition: def,
        });
    }
    Ok(out)
}

fn usage_err(e: sqlx::Error) -> CellarError {
    crate::connect::map_sqlx_err_for_runtime(e, "find usages", CellarError::introspection)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn def(kind: UsageKind, schema: &str, name: &str, definition: &str) -> UsageDefinition {
        UsageDefinition {
            kind,
            schema: schema.into(),
            name: name.into(),
            on_table: None,
            definition: definition.into(),
        }
    }

    #[test]
    fn search_confirms_real_references_and_skips_substrings() {
        let defs = vec![
            def(
                UsageKind::View,
                "public",
                "active_users",
                "SELECT * FROM users WHERE active",
            ),
            def(
                UsageKind::View,
                "public",
                "identity_map",
                "SELECT * FROM user_identities",
            ),
        ];
        let hits = search_usages(&defs, None, "public", "users", None);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "active_users");
    }

    #[test]
    fn search_respects_schema_filter() {
        let defs = vec![
            def(UsageKind::View, "public", "v1", "SELECT * FROM users"),
            def(UsageKind::View, "analytics", "v2", "SELECT * FROM users"),
        ];
        let scoped = search_usages(&defs, Some("public"), "public", "users", None);
        assert_eq!(scoped.len(), 1);
        assert_eq!(scoped[0].schema, "public");

        let all = search_usages(&defs, None, "public", "users", None);
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn search_column_scope_narrows_to_column() {
        let defs = vec![
            def(
                UsageKind::View,
                "public",
                "emails",
                "SELECT email FROM users",
            ),
            def(UsageKind::View, "public", "ids", "SELECT id FROM users"),
        ];
        let hits = search_usages(&defs, None, "public", "users", Some("email"));
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "emails");
        assert_eq!(hits[0].matched_column.as_deref(), Some("email"));
    }
}
