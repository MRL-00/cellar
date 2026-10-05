use crate::{changes, profiles, safety};
use std::time::Instant;

use cellar_sql::Dialect;
use futures::TryStreamExt;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::{
    postgres::{PgConnectOptions, PgConnection},
    sqlite::{SqliteConnectOptions, SqliteConnection},
    Column, Connection, Executor, Row, TypeInfo,
};

const BYTES: usize = 524_288;
const CELL_BYTES: usize = 16_384;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub action: String,
    pub connection: Option<String>,
    pub sql: Option<String>,
    pub schema: Option<String>,
    pub table: Option<String>,
    pub filters: Option<Vec<Filter>>,
    pub sort: Option<String>,
    pub descending: Option<bool>,
    pub offset: Option<u32>,
    pub limit: Option<u32>,
    pub profile: Option<Value>,
    pub password: Option<String>,
    pub session_profile: Option<profiles::Profile>,
    pub session_password: Option<String>,
    pub operation: Option<String>,
    pub changes: Option<Vec<changes::Change>>,
    pub preview_id: Option<String>,
    pub confirmation: Option<String>,
    pub destination: Option<String>,
    pub source: Option<String>,
    pub cursor: Option<usize>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Filter {
    column: String,
    operator: String,
    value: Option<String>,
}

pub async fn run(request: Request) -> Result<Value, String> {
    if request.session_profile.is_some()
        && !["schema", "browse", "query", "sql", "details", "plan"]
            .contains(&request.action.as_str())
    {
        return Err("Project connections are read-only".into());
    }
    if request.action == "import" {
        return snapshot(
            request
                .source
                .as_deref()
                .ok_or("Selected SQLite file required")?,
            request
                .destination
                .as_deref()
                .ok_or("Snapshot destination required")?,
        )
        .await;
    }
    if request.action == "connection" {
        return profiles::manage(
            request.operation.as_deref().unwrap_or("save"),
            request.profile,
            request.connection.as_deref(),
            request.password.as_deref(),
        );
    }
    let mut configs = profiles::list()?;
    if let Some(mut profile) = request.session_profile.clone() {
        if Some(&profile.config.id) != request.connection.as_ref()
            || !profile.config.id.starts_with("project-")
        {
            return Err("Invalid project session".into());
        }
        profile.allow_edits = false;
        profile.managed = true;
        profile.has_password = false;
        if profile.config.engine.family() == cellar_core::driver::Engine::Postgres
            && !["127.0.0.1", "localhost", "::1"].contains(&profile.config.host.as_str())
        {
            profile.config.ssl_mode = cellar_core::driver::SslMode::VerifyFull;
        }
        configs.insert(0, profile);
    }
    if request.action == "connections" {
        return Ok(profiles::public(&configs));
    }
    let config = configs
        .iter()
        .find(|profile| Some(&profile.config.id) == request.connection.as_ref())
        .ok_or("Unknown connection alias")?;
    if request.action == "commit" && !config.allow_edits {
        return Err("Enable editing in this connection's settings before committing".into());
    }
    if request.action == "sql" {
        let sql = request.sql.as_deref().unwrap_or_default();
        let statements = cellar_sql::split_statements(sql);
        if statements.len() > 10 {
            return Err("Run at most 10 statements at a time".into());
        }
        let selected = if request.operation.as_deref() == Some("statement") {
            cellar_sql::statement_at_offset(sql, request.cursor.unwrap_or(0))
                .map(|s| vec![s])
                .unwrap_or_default()
        } else {
            statements
        };
        let postgres = config.config.engine.family() == cellar_core::driver::Engine::Postgres;
        let queries = selected
            .iter()
            .map(|s| safety::validate(s.text, postgres))
            .collect::<Result<Vec<_>, _>>()?;
        if queries.is_empty() {
            return Err("No SQL statement selected".into());
        }
        return Ok(json!({"statements":queries}));
    }
    match config.config.engine.family() {
        cellar_core::driver::Engine::Sqlite => sqlite(config, &request).await,
        cellar_core::driver::Engine::Postgres => postgres(config, &request).await,
        _ => Err("Only PostgreSQL and SQLite are supported".into()),
    }
}

fn browse_sql(request: &Request, columns: &[String], dialect: Dialect) -> Result<String, String> {
    let schema = request.schema.as_deref().unwrap_or("main");
    let table = request.table.as_deref().ok_or("Select a table")?;
    let mut sql = format!("SELECT * FROM {}", dialect.quote_qualified(schema, table));
    let filters = request.filters.as_deref().unwrap_or_default();
    if filters.len() > 12 {
        return Err("At most 12 filters are supported".into());
    }
    for (index, filter) in filters.iter().enumerate() {
        if !columns.contains(&filter.column) {
            return Err("Unknown filter column".into());
        }
        sql.push_str(if index == 0 { " WHERE " } else { " AND " });
        let column = dialect.quote_ident(&filter.column);
        let value = filter.value.as_deref().unwrap_or_default();
        if value.len() > 1024 {
            return Err("Filter value exceeds 1024 bytes".into());
        }
        let literal = cellar_sql::quote_string_literal(value);
        let clause = match filter.operator.as_str() {
            "equals" => format!("{column} = {literal}"),
            "notEquals" => format!("{column} <> {literal}"),
            "greaterThan" => format!("{column} > {literal}"),
            "greaterThanOrEqual" => format!("{column} >= {literal}"),
            "lessThan" => format!("{column} < {literal}"),
            "lessThanOrEqual" => format!("{column} <= {literal}"),
            "like" => format!("LOWER(CAST({column} AS TEXT)) LIKE LOWER({literal})"),
            "contains" | "notContains" | "startsWith" | "endsWith" => {
                let escaped = value
                    .replace('!', "!!")
                    .replace('%', "!%")
                    .replace('_', "!_");
                let pattern = match filter.operator.as_str() {
                    "startsWith" => format!("{escaped}%"),
                    "endsWith" => format!("%{escaped}"),
                    _ => format!("%{escaped}%"),
                };
                format!(
                    "LOWER(CAST({column} AS TEXT)) {}LIKE LOWER({}) ESCAPE '!'",
                    if filter.operator == "notContains" {
                        "NOT "
                    } else {
                        ""
                    },
                    cellar_sql::quote_string_literal(&pattern)
                )
            }
            "isNull" => format!("{column} IS NULL"),
            "isNotNull" => format!("{column} IS NOT NULL"),
            _ => return Err("Unsupported filter operator".into()),
        };
        sql.push_str(&clause);
    }
    if let Some(sort) = &request.sort {
        if !columns.contains(sort) {
            return Err("Unknown sort column".into());
        }
        sql.push_str(&format!(
            " ORDER BY {} {}",
            dialect.quote_ident(sort),
            if request.descending.unwrap_or(false) {
                "DESC"
            } else {
                "ASC"
            }
        ));
    } else if let Some(first) = columns.first() {
        sql.push_str(&format!(" ORDER BY {}", dialect.quote_ident(first)));
    }
    let offset = request.offset.unwrap_or(0);
    if offset > 100_000 {
        return Err("Offset exceeds 100,000 rows".into());
    }
    sql.push_str(&format!(" LIMIT {} OFFSET {offset}", limit(request) + 1));
    Ok(sql)
}

fn limit(request: &Request) -> usize {
    request.limit.unwrap_or(100).clamp(1, 500) as usize
}

fn bounded(value: Value) -> Value {
    match value {
        Value::String(mut text) if text.len() > CELL_BYTES => {
            let mut end = CELL_BYTES;
            while !text.is_char_boundary(end) {
                end -= 1;
            }
            text.truncate(end);
            Value::String(format!("{text}… [cell truncated]"))
        }
        other => other,
    }
}

async fn sqlite(profile: &profiles::Profile, request: &Request) -> Result<Value, String> {
    let config = &profile.config;
    let writing = request.action == "commit" && profile.allow_edits;
    let options = SqliteConnectOptions::new()
        .filename(&config.database)
        .read_only(!writing)
        .create_if_missing(false)
        .pragma("query_only", if writing { "OFF" } else { "ON" })
        .pragma("foreign_keys", "ON")
        .pragma("trusted_schema", "OFF");
    let mut connection = SqliteConnection::connect_with(&options)
        .await
        .map_err(|_| "SQLite connection failed; verify the local read-only file alias")?;
    let started = Instant::now();
    if request.action == "details" {
        return crate::metadata::sqlite(&mut connection, request).await;
    }
    if request.action == "snapshot" {
        return snapshot(
            &config.database,
            request
                .destination
                .as_deref()
                .ok_or("Snapshot destination required")?,
        )
        .await;
    }
    if request.action == "preview" || request.action == "commit" {
        return changes::sqlite(&mut connection, request, writing).await;
    }
    if request.action == "schema" {
        let tables = sqlx::query("SELECT name, type FROM sqlite_schema WHERE type IN ('table','view') AND name NOT LIKE 'sqlite_%' ORDER BY name LIMIT 201")
            .fetch_all(&mut connection).await.map_err(|_| "Schema read failed")?;
        if tables.len() > 200 {
            return Err("Schema exceeds 200 tables; narrow the database alias".into());
        }
        let mut result = Vec::new();
        for table in tables {
            let name: String = table.get(0);
            let rows = sqlx::query(
                "SELECT name, type, \"notnull\", pk FROM pragma_table_info(?) LIMIT 129",
            )
            .bind(&name)
            .fetch_all(&mut connection)
            .await
            .map_err(|_| "Column read failed")?;
            if rows.len() > 128 {
                return Err("Table exceeds 128 columns".into());
            }
            result.push(json!({"schema":"main", "name":name, "kind":table.get::<String,_>(1),
                "columns":rows.iter().map(|row| json!({"name":row.get::<String,_>(0), "type":row.get::<String,_>(1),
                    "nullable":row.get::<i64,_>(2)==0, "primaryKey":row.get::<i64,_>(3)>0})).collect::<Vec<_>>()}));
        }
        return Ok(json!({"tables":result}));
    }
    let sql = if request.action == "browse" {
        if request.schema.as_deref().unwrap_or("main") != "main" {
            return Err("Unknown schema".into());
        }
        let table = request.table.as_deref().ok_or("Select a table")?;
        let rows = sqlx::query("SELECT name FROM pragma_table_info(?) LIMIT 129")
            .bind(table)
            .fetch_all(&mut connection)
            .await
            .map_err(|_| "Table metadata unavailable")?;
        let columns = rows
            .iter()
            .map(|row| row.get::<String, _>(0))
            .collect::<Vec<_>>();
        if columns.is_empty() || columns.len() > 128 {
            return Err("Table unavailable or too wide".into());
        }
        browse_sql(request, &columns, Dialect::Sqlite)?
    } else if request.action == "query" || request.action == "plan" {
        safety::validate(request.sql.as_deref().unwrap_or_default(), false)?
    } else {
        return Err("Unknown action".into());
    };
    let sql = if request.action == "plan" {
        format!("EXPLAIN QUERY PLAN {sql}")
    } else {
        sql
    };
    let description = connection
        .describe(&sql)
        .await
        .map_err(|_| "SQL validation failed")?;
    let columns = description
        .columns()
        .iter()
        .map(|c| json!({"name":c.name(), "type":c.type_info().name()}))
        .collect::<Vec<_>>();
    if columns.len() > 128 {
        return Err("Result exceeds 128 columns".into());
    }
    let mut stream = sqlx::query(&sql).fetch(&mut connection);
    let mut rows = Vec::new();
    let mut used = serde_json::to_vec(&columns).unwrap().len();
    let mut truncated = false;
    let mut revisions = Vec::new();
    while let Some(row) = stream
        .try_next()
        .await
        .map_err(|_| "Query failed or was interrupted")?
    {
        if rows.len() >= limit(request) {
            truncated = true;
            break;
        }
        let original = sqlite_cells(&row);
        let revision = changes::revision(&original);
        let cells = original.into_iter().map(bounded).collect::<Vec<_>>();
        used += serde_json::to_vec(&cells).unwrap().len();
        if used > BYTES {
            truncated = true;
            break;
        }
        rows.push(cells);
        revisions.push(revision);
    }
    Ok(
        json!({"columns":columns, "rows":rows, "truncated":truncated,
        "durationMs":started.elapsed().as_millis(), "readOnly":!profile.allow_edits, "revisions":revisions}),
    )
}

async fn postgres(profile: &profiles::Profile, request: &Request) -> Result<Value, String> {
    let config = &profile.config;
    let writing = request.action == "commit" && profile.allow_edits;
    if !["127.0.0.1", "localhost", "::1"].contains(&config.host.as_str())
        && !config.host.starts_with('/')
        && !profile.managed
    {
        return Err(
            "This local build only accepts loopback or Unix-socket Postgres aliases".into(),
        );
    }
    let mut options = PgConnectOptions::new()
        .host(&config.host)
        .port(config.port)
        .database(&config.database)
        .username(&config.user)
        .application_name("cellar-extension-read-only")
        .options([
            (
                "default_transaction_read_only",
                if writing { "off" } else { "on" },
            ),
            ("statement_timeout", "4000"),
            ("lock_timeout", "1000"),
        ]);
    if profile.managed {
        options = options.ssl_mode(match config.ssl_mode {
            cellar_core::driver::SslMode::Disable => sqlx::postgres::PgSslMode::Disable,
            cellar_core::driver::SslMode::Prefer => sqlx::postgres::PgSslMode::Prefer,
            cellar_core::driver::SslMode::Require => sqlx::postgres::PgSslMode::Require,
            cellar_core::driver::SslMode::VerifyCa => sqlx::postgres::PgSslMode::VerifyCa,
            cellar_core::driver::SslMode::VerifyFull => sqlx::postgres::PgSslMode::VerifyFull,
        });
        if let Some(pem) = &profile.ssl_ca_pem {
            options = options.ssl_root_cert_from_pem(pem.as_bytes().to_vec());
        }
        if profile.has_password {
            if let Some(password) = cellar_secrets::load(&format!("openai-extension:{}", config.id))
                .map_err(|_| "Cannot read this connection's OS keychain credential")?
            {
                options = options.password(&password);
            }
        }
    }
    if request.session_profile.is_some() {
        if let Some(password) = &request.session_password {
            options = options.password(password);
        }
    }
    let mut connection = PgConnection::connect_with(&options)
        .await
        .map_err(|_| "Postgres connection failed; verify the local nonsecret alias")?;
    let mut transaction = connection
        .begin()
        .await
        .map_err(|_| "Cannot begin read-only transaction")?;
    sqlx::query(if writing {
        "SET TRANSACTION READ WRITE"
    } else {
        "SET TRANSACTION READ ONLY"
    })
    .execute(&mut *transaction)
    .await
    .map_err(|_| "Cannot enforce read-only transaction")?;
    sqlx::query("SET LOCAL search_path = pg_catalog, public")
        .execute(&mut *transaction)
        .await
        .map_err(|_| "Cannot restrict search path")?;
    sqlx::query("SET LOCAL standard_conforming_strings = on")
        .execute(&mut *transaction)
        .await
        .map_err(|_| "Cannot enforce SQL literal escaping")?;
    let started = Instant::now();
    if request.action == "details" {
        let value = crate::metadata::postgres(&mut transaction, request).await?;
        transaction
            .rollback()
            .await
            .map_err(|_| "Read transaction cleanup failed")?;
        return Ok(value);
    }
    if request.action == "preview" || request.action == "commit" {
        return changes::postgres(transaction, request, writing).await;
    }
    if request.action == "schema" {
        let rows = sqlx::query("SELECT col.table_schema, col.table_name, col.column_name, col.data_type, col.is_nullable, tab.table_type, EXISTS(SELECT 1 FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace JOIN pg_attribute a ON a.attrelid=c.oid JOIN pg_index i ON i.indrelid=c.oid WHERE n.nspname=col.table_schema AND c.relname=col.table_name AND a.attname=col.column_name AND i.indisprimary AND a.attnum=ANY(i.indkey)) FROM information_schema.columns col JOIN information_schema.tables tab ON tab.table_schema=col.table_schema AND tab.table_name=col.table_name WHERE col.table_schema NOT IN ('pg_catalog','information_schema') ORDER BY col.table_schema,col.table_name,col.ordinal_position LIMIT 4097")
            .fetch_all(&mut *transaction).await.map_err(|_| "Schema read failed")?;
        if rows.len() > 4096 {
            return Err("Schema exceeds 4096 columns; narrow the database alias".into());
        }
        let mut tables: Vec<Value> = Vec::new();
        for row in rows {
            let schema: String = row.get(0);
            let name: String = row.get(1);
            if tables
                .last()
                .is_none_or(|t| t["schema"] != schema || t["name"] != name)
            {
                if tables.len() >= 200 {
                    return Err("Schema exceeds 200 tables".into());
                }
                tables.push(json!({"schema":schema,"name":name,"kind":if row.get::<String,_>(5)=="VIEW" {"view"}else{"table"},"columns":[]}));
            }
            let columns = tables.last_mut().unwrap()["columns"]
                .as_array_mut()
                .unwrap();
            if columns.len() >= 128 {
                return Err("Table exceeds 128 columns".into());
            }
            columns.push(
                json!({"name":row.get::<String,_>(2), "type":row.get::<String,_>(3),
                "nullable":row.get::<String,_>(4)=="YES","primaryKey":row.get::<bool,_>(6)}),
            );
        }
        transaction
            .rollback()
            .await
            .map_err(|_| "Read transaction cleanup failed")?;
        return Ok(json!({"tables":tables}));
    }
    let sql = if request.action == "browse" {
        let rows = sqlx::query("SELECT column_name FROM information_schema.columns WHERE table_schema=$1 AND table_name=$2 ORDER BY ordinal_position LIMIT 129")
            .bind(request.schema.as_deref().unwrap_or("public")).bind(request.table.as_deref().unwrap_or_default())
            .fetch_all(&mut *transaction).await.map_err(|_| "Table metadata unavailable")?;
        let columns = rows
            .iter()
            .map(|row| row.get::<String, _>(0))
            .collect::<Vec<_>>();
        if columns.is_empty() || columns.len() > 128 {
            return Err("Table unavailable or too wide".into());
        }
        browse_sql(request, &columns, Dialect::Postgres)?
    } else if request.action == "query" || request.action == "plan" {
        safety::validate(request.sql.as_deref().unwrap_or_default(), true)?
    } else {
        return Err("Unknown action".into());
    };
    if request.action == "plan" {
        let row = sqlx::query(&format!("EXPLAIN (FORMAT JSON) {sql}"))
            .fetch_one(&mut *transaction)
            .await
            .map_err(|_| "Query plan unavailable")?;
        let plan: Value = row.get(0);
        if serde_json::to_vec(&plan)
            .map_err(|_| "Cannot encode query plan")?
            .len()
            > BYTES
        {
            return Err("Query plan exceeds response budget".into());
        }
        transaction
            .rollback()
            .await
            .map_err(|_| "Read transaction cleanup failed")?;
        return Ok(json!({"plan":plan,"durationMs":started.elapsed().as_millis()}));
    }
    let description = (&mut *transaction)
        .describe(&sql)
        .await
        .map_err(|_| "SQL validation failed")?;
    let columns = description
        .columns()
        .iter()
        .map(|c| json!({"name":c.name(), "type":c.type_info().name()}))
        .collect::<Vec<_>>();
    if columns.len() > 128 {
        return Err("Result exceeds 128 columns".into());
    }
    let count = limit(request) + 1;
    let names = columns
        .iter()
        .map(|column| column["name"].as_str().unwrap())
        .collect::<std::collections::HashSet<_>>();
    if names.len() != columns.len() {
        return Err("Use unique column aliases in the SQL result".into());
    }
    let query =
        format!("SELECT to_jsonb(cellar_row)::text FROM ({sql}) AS cellar_row LIMIT {count}");
    let mut stream = sqlx::query(&query).fetch(&mut *transaction);
    let mut rows = Vec::new();
    let mut used = serde_json::to_vec(&columns).unwrap().len();
    let mut truncated = false;
    let mut revisions = Vec::new();
    while let Some(row) = stream
        .try_next()
        .await
        .map_err(|_| "Query failed or was interrupted")?
    {
        if rows.len() >= limit(request) {
            truncated = true;
            break;
        }
        let text: String = row.get(0);
        if text.len() > BYTES {
            truncated = true;
            break;
        }
        let object: Value = serde_json::from_str(&text).map_err(|_| "Result decoding failed")?;
        let original = postgres_cells(
            &object,
            &columns
                .iter()
                .map(|c| c["name"].as_str().unwrap().to_string())
                .collect::<Vec<_>>(),
        );
        let revision = changes::revision(&original);
        let cells = original.into_iter().map(bounded).collect::<Vec<_>>();
        used += serde_json::to_vec(&cells).unwrap().len();
        if used > BYTES {
            truncated = true;
            break;
        }
        rows.push(cells);
        revisions.push(revision);
    }
    drop(stream);
    transaction
        .rollback()
        .await
        .map_err(|_| "Read transaction cleanup failed")?;
    Ok(json!({"columns":columns,"rows":rows,"truncated":truncated,
        "durationMs":started.elapsed().as_millis(),"readOnly":!profile.allow_edits,"revisions":revisions}))
}

pub fn sqlite_cells(row: &sqlx::sqlite::SqliteRow) -> Vec<Value> {
    (0..row.len())
        .map(|index| {
            if let Ok(value) = row.try_get::<Option<i64>, _>(index) {
                return value.map(|v| json!(v)).unwrap_or(Value::Null);
            }
            if let Ok(value) = row.try_get::<Option<f64>, _>(index) {
                return value.map(|v| json!(v)).unwrap_or(Value::Null);
            }
            if let Ok(value) = row.try_get::<Option<String>, _>(index) {
                return value.map(Value::String).unwrap_or(Value::Null);
            }
            json!("[binary or unsupported value]")
        })
        .collect()
}

pub fn postgres_cells(object: &Value, columns: &[String]) -> Vec<Value> {
    columns
        .iter()
        .map(
            |name| match object.get(name).cloned().unwrap_or(Value::Null) {
                value @ (Value::Object(_) | Value::Array(_)) => json!(value.to_string()),
                other => other,
            },
        )
        .collect()
}

async fn snapshot(source: &str, destination: &str) -> Result<Value, String> {
    let root = profiles::directory()?;
    let destination = std::path::Path::new(destination);
    let parent = destination
        .parent()
        .and_then(|p| std::fs::canonicalize(p).ok())
        .ok_or("Invalid snapshot destination")?;
    let allowed = [root.join("databases"), root.join("exports")]
        .iter()
        .filter_map(|p| std::fs::canonicalize(p).ok())
        .any(|p| p == parent);
    if !allowed || destination.exists() {
        return Err("Snapshot destination must be a new private file".into());
    }
    let source = std::fs::canonicalize(source).map_err(|_| "Selected database unavailable")?;
    if !std::fs::metadata(&source)
        .map_err(|_| "Selected database unavailable")?
        .is_file()
    {
        return Err("Select a database file".into());
    }
    let mut connection = SqliteConnection::connect_with(
        &SqliteConnectOptions::new()
            .filename(source)
            .read_only(true)
            .create_if_missing(false)
            .pragma("trusted_schema", "OFF"),
    )
    .await
    .map_err(|_| "Cannot open selected SQLite database")?;
    sqlx::query(&format!(
        "VACUUM INTO {}",
        cellar_sql::quote_string_literal(destination.to_str().ok_or("Invalid snapshot filename")?)
    ))
    .execute(&mut connection)
    .await
    .map_err(|_| "Cannot create consistent database snapshot")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(destination, std::fs::Permissions::from_mode(0o600))
            .map_err(|_| "Cannot protect database snapshot")?;
    }
    Ok(json!({"snapshot":true}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn filters_quote_names_values_and_wildcards() {
        let request: Request = serde_json::from_value(json!({"action":"browse", "table":"a\"b",
            "filters":[{"column":"name", "operator":"contains", "value":"x%' OR 1=1 --"}]}))
        .unwrap();
        let sql = browse_sql(&request, &["name".into()], Dialect::Sqlite).unwrap();
        assert!(sql.contains("\"a\"\"b\""));
        assert!(sql.contains("!%'' OR 1=1 --"));
        assert!(safety::validate(&sql, false).is_ok());
    }

    #[tokio::test]
    async fn project_session_rejects_writes_before_profile_loading() {
        let profile = json!({"id":"project-synthetic","name":"Synthetic","engine":"sqlite",
            "host":"","port":5432,"database":"/synthetic/unused.sqlite","user":"",
            "ssl_mode":"disable","env_tag":"local","application_name":null,"color":null,
            "allow_edits":true,"managed":true,"has_password":false});
        let request: Request = serde_json::from_value(json!({"action":"commit",
            "connection":"project-synthetic","session_profile":profile}))
        .unwrap();
        assert_eq!(
            run(request).await.unwrap_err(),
            "Project connections are read-only"
        );
    }

    #[tokio::test]
    async fn sqlite_engine_refuses_writes_even_without_parser() {
        let path = env::temp_dir().join(format!("cellar-extension-{}.sqlite", std::process::id()));
        let mut writable = SqliteConnection::connect_with(
            &SqliteConnectOptions::new()
                .filename(&path)
                .create_if_missing(true),
        )
        .await
        .unwrap();
        writable
            .execute("CREATE TABLE sample(id INTEGER)")
            .await
            .unwrap();
        writable.close().await.unwrap();
        let mut readonly = SqliteConnection::connect_with(
            &SqliteConnectOptions::new()
                .filename(&path)
                .read_only(true)
                .pragma("query_only", "ON"),
        )
        .await
        .unwrap();
        assert!(readonly
            .execute("INSERT INTO sample VALUES (1)")
            .await
            .is_err());
        readonly.close().await.unwrap();
        std::fs::remove_file(path).unwrap();
    }
}
