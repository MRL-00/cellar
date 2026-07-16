//! Azure Cosmos DB (NoSQL / Core SQL API) driver for Cellar.
//!
//! This first slice treats Cosmos containers as read-only tables under a
//! `documents` schema, mirroring the Firestore and Convex drivers. Columns are
//! inferred from a small document sample. Auth is the account primary key via
//! the REST data-plane HMAC signature — Entra ID is a follow-up.

use std::any::Any;
use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use cellar_core::driver::{Connection, ConnectionConfig, Driver, DriverInfo, Engine, SslMode};
use cellar_core::error::{CellarError, CellarResult};
use cellar_core::query::{
    NoticeCapture, PlanMode, Query, QueryPlan, QueryResult, SortDirection, TableBrowseRequest,
    TableFilterClause, TableFilterOperator, TableSortClause,
};
use cellar_core::schema::{Column, Database, Index, Schema, Table};
use cellar_core::value::{CellValue, ColumnMeta, Row};
use hmac::{Hmac, Mac};
use reqwest::{Client, Method, RequestBuilder, Response, Url};
use serde::Deserialize;
use serde_json::{json, Map, Value};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

const SCHEMA_NAME: &str = "documents";
const DEFAULT_SAMPLE_SIZE: usize = 25;
const DEFAULT_BROWSE_LIMIT: u32 = 500;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
/// Stable REST API version with cross-partition query support.
const API_VERSION: &str = "2018-12-31";

#[derive(Debug, Default, Clone, Copy)]
pub struct CosmosDriver;

pub async fn browse_table(
    conn: &dyn Connection,
    request: &TableBrowseRequest,
    table: &Table,
) -> CellarResult<QueryResult> {
    let cx = as_cosmos(conn)?;
    cx.browse_table(request, table).await
}

#[async_trait]
impl Driver for CosmosDriver {
    fn engine(&self) -> Engine {
        Engine::Cosmos
    }

    async fn connect(
        &self,
        config: &ConnectionConfig,
        secret: Option<&str>,
    ) -> CellarResult<Box<dyn Connection>> {
        let connection = CosmosConnection::open(config, secret).await?;
        Ok(Box::new(connection))
    }

    async fn introspect(&self, conn: &dyn Connection) -> CellarResult<Vec<Database>> {
        let cx = as_cosmos(conn)?;
        cx.introspect().await
    }

    async fn execute_query(
        &self,
        _conn: &dyn Connection,
        _query: &Query,
    ) -> CellarResult<QueryResult> {
        Err(CellarError::query(
            "Cosmos SQL query execution is not supported yet; browse containers from the sidebar",
        ))
    }

    async fn explain_query(
        &self,
        _conn: &dyn Connection,
        _query: &Query,
        _mode: PlanMode,
    ) -> CellarResult<QueryPlan> {
        Err(CellarError::invalid_config(
            "Cosmos does not expose SQL execution plans through Cellar yet",
        ))
    }
}

pub struct CosmosConnection {
    info: DriverInfo,
    client: Client,
    base_url: Url,
    /// When set, introspection is scoped to this Cosmos database.
    scoped_database: Option<String>,
    master_key: Vec<u8>,
}

impl CosmosConnection {
    async fn open(config: &ConnectionConfig, secret: Option<&str>) -> CellarResult<Self> {
        let key = secret
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                CellarError::invalid_config(
                    "Cosmos primary key is required (paste the account key as the password)",
                )
            })?;
        let master_key = BASE64.decode(key).map_err(|_| {
            CellarError::invalid_config(
                "Cosmos primary key must be a valid Base64 account key from the Azure portal",
            )
        })?;
        if config.host.trim().is_empty() {
            return Err(CellarError::invalid_config(
                "Cosmos account endpoint is required, e.g. myaccount.documents.azure.com",
            ));
        }
        let client = Client::builder()
            .user_agent(user_agent(config))
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|e| CellarError::connection(e.to_string()))?;
        let base_url = cosmos_base_url(config)?;
        let account = account_name(config);
        let scoped_database = {
            let db = config.database.trim();
            if db.is_empty() {
                None
            } else {
                Some(db.to_string())
            }
        };
        let connection = Self {
            info: DriverInfo {
                engine: Engine::Cosmos,
                version: format!("Cosmos DB NoSQL REST API ({account})"),
            },
            client,
            base_url,
            scoped_database,
            master_key,
        };
        // Validate credentials by listing databases (or reading the scoped one).
        connection.list_database_ids().await?;
        Ok(connection)
    }

    async fn introspect(&self) -> CellarResult<Vec<Database>> {
        let database_ids = self.list_database_ids().await?;
        let mut out = Vec::with_capacity(database_ids.len());

        for (idx, db_id) in database_ids.into_iter().enumerate() {
            let containers = self.list_containers(&db_id).await?;
            let mut tables = Vec::with_capacity(containers.len());
            for container_id in containers {
                let sample = self
                    .query_documents_page(
                        &db_id,
                        &container_id,
                        DEFAULT_SAMPLE_SIZE as i32,
                        None,
                        &CosmosQuery {
                            sql: "SELECT * FROM c".into(),
                            parameters: vec![],
                        },
                    )
                    .await?
                    .documents;
                let columns = infer_columns(&sample);
                tables.push(Table {
                    name: container_id,
                    schema: SCHEMA_NAME.into(),
                    row_count: None,
                    columns,
                    primary_key: vec!["id".into()],
                    foreign_keys: vec![],
                    indexes: vec![Index {
                        name: "id".into(),
                        columns: vec!["id".into()],
                        unique: true,
                        primary: true,
                    }],
                });
            }
            out.push(Database {
                name: db_id,
                is_default: idx == 0,
                schemas: vec![Schema {
                    name: SCHEMA_NAME.into(),
                    tables,
                    views: vec![],
                }],
            });
        }

        Ok(out)
    }

    async fn browse_table(
        &self,
        request: &TableBrowseRequest,
        table: &Table,
    ) -> CellarResult<QueryResult> {
        if request.schema != SCHEMA_NAME {
            return Err(CellarError::invalid_config(format!(
                "Cosmos table schema must be {SCHEMA_NAME}"
            )));
        }

        let database_id = request.database.as_deref().ok_or_else(|| {
            CellarError::invalid_config("Cosmos browse requires a target database")
        })?;
        let query = build_browse_query(table, &request.filters, &request.sorts)?;

        let started = Instant::now();
        let limit = request.limit.unwrap_or(DEFAULT_BROWSE_LIMIT) as usize;
        // Continuation has no server-side OFFSET, so paging re-reads from the
        // start and slices the requested window locally (same as Convex).
        let offset = request.offset.unwrap_or(0) as usize;
        let fetch_until = offset.saturating_add(limit);
        let mut documents: Vec<Map<String, Value>> = Vec::new();
        let mut continuation: Option<String> = None;
        let mut truncated = false;

        loop {
            let page_size = ((fetch_until.saturating_sub(documents.len())).max(1) as i32)
                .clamp(1, DEFAULT_BROWSE_LIMIT as i32);
            let page = self
                .query_documents_page(
                    database_id,
                    &request.table,
                    page_size,
                    continuation.as_deref(),
                    &query,
                )
                .await?;
            continuation = page.continuation;
            documents.extend(page.documents);
            if documents.len() >= fetch_until {
                truncated = documents.len() > fetch_until || continuation.is_some();
                documents.truncate(fetch_until);
                break;
            }
            if continuation.is_none() {
                break;
            }
        }

        let start = offset.min(documents.len());
        let mut documents = documents.split_off(start);
        // Page-local sort only — see build_browse_query for why ORDER BY stays
        // off the wire.
        sort_documents(&mut documents, &request.sorts);
        let columns = columns_for_browse(table, &documents);
        let rows = documents
            .iter()
            .map(|doc| row_for_document(doc, &columns))
            .collect::<Vec<_>>();

        Ok(QueryResult {
            columns: columns
                .into_iter()
                .map(|column| ColumnMeta {
                    name: column.name,
                    data_type: column.data_type,
                    nullable: column.nullable,
                })
                .collect(),
            rows,
            notices: vec![],
            notice_capture: NoticeCapture::unsupported(
                "Cosmos HTTP responses do not include database notice frames",
            ),
            rows_affected: None,
            duration_ms: started.elapsed().as_millis() as u64,
            truncated,
            total_rows: None,
        })
    }

    async fn list_database_ids(&self) -> CellarResult<Vec<String>> {
        if let Some(scoped) = &self.scoped_database {
            // Confirm the scoped database exists.
            let _ = self.get_database(scoped).await?;
            return Ok(vec![scoped.clone()]);
        }
        let response = self
            .send(
                Method::GET,
                self.dbs_url()?,
                "dbs",
                "",
                None,
                RequestBody::None,
            )
            .await?;
        let feed: DatabasesFeed = response
            .json()
            .await
            .map_err(|e| CellarError::decode(e.to_string()))?;
        Ok(feed.databases.into_iter().map(|d| d.id).collect())
    }

    async fn get_database(&self, database_id: &str) -> CellarResult<NamedResource> {
        let response = self
            .send(
                Method::GET,
                self.db_url(database_id)?,
                "dbs",
                format!("dbs/{database_id}"),
                None,
                RequestBody::None,
            )
            .await?;
        response
            .json()
            .await
            .map_err(|e| CellarError::decode(e.to_string()))
    }

    async fn list_containers(&self, database_id: &str) -> CellarResult<Vec<String>> {
        let response = self
            .send(
                Method::GET,
                self.colls_url(database_id)?,
                "colls",
                format!("dbs/{database_id}"),
                None,
                RequestBody::None,
            )
            .await?;
        let feed: CollectionsFeed = response
            .json()
            .await
            .map_err(|e| CellarError::decode(e.to_string()))?;
        Ok(feed.collections.into_iter().map(|c| c.id).collect())
    }

    async fn query_documents_page(
        &self,
        database_id: &str,
        container_id: &str,
        max_items: i32,
        continuation: Option<&str>,
        query: &CosmosQuery,
    ) -> CellarResult<DocumentsPage> {
        let mut extra = vec![
            ("x-ms-documentdb-isquery", "True".into()),
            ("x-ms-documentdb-query-enablecrosspartition", "True".into()),
            ("x-ms-max-item-count", max_items.to_string()),
            ("Content-Type", "application/query+json".into()),
        ];
        if let Some(token) = continuation {
            extra.push(("x-ms-continuation", token.to_string()));
        }
        let body = json!({
            "query": query.sql,
            "parameters": query.parameters,
        });
        let response = self
            .send(
                Method::POST,
                self.docs_url(database_id, container_id)?,
                "docs",
                format!("dbs/{database_id}/colls/{container_id}"),
                Some(&extra),
                RequestBody::Json(body),
            )
            .await?;
        let next_continuation = response
            .headers()
            .get("x-ms-continuation")
            .and_then(|v| v.to_str().ok())
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let feed: DocumentsFeed = response
            .json()
            .await
            .map_err(|e| CellarError::decode(e.to_string()))?;
        Ok(DocumentsPage {
            documents: feed.documents,
            continuation: next_continuation,
        })
    }

    fn dbs_url(&self) -> CellarResult<Url> {
        let mut url = self.base_url.clone();
        url.path_segments_mut()
            .map_err(|_| CellarError::invalid_config("invalid Cosmos endpoint URL"))?
            .clear()
            .push("dbs");
        Ok(url)
    }

    fn db_url(&self, database_id: &str) -> CellarResult<Url> {
        let mut url = self.base_url.clone();
        url.path_segments_mut()
            .map_err(|_| CellarError::invalid_config("invalid Cosmos endpoint URL"))?
            .clear()
            .extend(["dbs", database_id]);
        Ok(url)
    }

    fn colls_url(&self, database_id: &str) -> CellarResult<Url> {
        let mut url = self.base_url.clone();
        url.path_segments_mut()
            .map_err(|_| CellarError::invalid_config("invalid Cosmos endpoint URL"))?
            .clear()
            .extend(["dbs", database_id, "colls"]);
        Ok(url)
    }

    fn docs_url(&self, database_id: &str, container_id: &str) -> CellarResult<Url> {
        let mut url = self.base_url.clone();
        url.path_segments_mut()
            .map_err(|_| CellarError::invalid_config("invalid Cosmos endpoint URL"))?
            .clear()
            .extend(["dbs", database_id, "colls", container_id, "docs"]);
        Ok(url)
    }

    async fn send(
        &self,
        method: Method,
        url: Url,
        resource_type: &str,
        resource_link: impl AsRef<str>,
        extra_headers: Option<&[(&str, String)]>,
        body: RequestBody,
    ) -> CellarResult<Response> {
        let date = rfc1123_now();
        let authorization = master_key_authorization(
            method.as_str(),
            resource_type,
            resource_link.as_ref(),
            &date,
            &self.master_key,
        )?;
        let mut request: RequestBuilder = self
            .client
            .request(method.clone(), url)
            .header("Authorization", authorization)
            .header("x-ms-date", &date)
            .header("x-ms-version", API_VERSION)
            .header("Accept", "application/json");
        if let Some(headers) = extra_headers {
            for (name, value) in headers {
                request = request.header(*name, value);
            }
        }
        request = match body {
            RequestBody::None => request,
            RequestBody::Json(value) => request.json(&value),
        };
        let response = request
            .send()
            .await
            .map_err(|e| CellarError::connection(e.to_string()))?;
        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(map_cosmos_status(status.as_u16(), text));
        }
        Ok(response)
    }
}

#[async_trait]
impl Connection for CosmosConnection {
    fn info(&self) -> &DriverInfo {
        &self.info
    }

    async fn ping(&self) -> CellarResult<()> {
        self.list_database_ids().await.map(|_| ())
    }

    async fn close(&self) -> CellarResult<()> {
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

enum RequestBody {
    None,
    Json(Value),
}

#[derive(Debug, Deserialize)]
struct NamedResource {
    id: String,
}

#[derive(Debug, Deserialize)]
struct DatabasesFeed {
    #[serde(default, rename = "Databases")]
    databases: Vec<NamedResource>,
}

#[derive(Debug, Deserialize)]
struct CollectionsFeed {
    #[serde(default, rename = "DocumentCollections")]
    collections: Vec<NamedResource>,
}

#[derive(Debug, Deserialize)]
struct DocumentsFeed {
    #[serde(default, rename = "Documents")]
    documents: Vec<Map<String, Value>>,
}

struct DocumentsPage {
    documents: Vec<Map<String, Value>>,
    continuation: Option<String>,
}

#[derive(Debug, Clone)]
struct CosmosQuery {
    sql: String,
    parameters: Vec<Value>,
}

/// Build a parameterized Cosmos SQL query from grid filter clauses.
///
/// Nested JSON columns use `ToString(c["field"])` for text operators so
/// `contains` finds values inside the document blob.
///
/// Sorts are intentionally omitted: the Cosmos REST gateway cannot serve
/// cross-partition `ORDER BY` (it returns a 400 that only full SDKs handle by
/// fanning out per partition). Callers should sort the returned page locally.
fn build_browse_query(
    table: &Table,
    filters: &[TableFilterClause],
    _sorts: &[TableSortClause],
) -> CellarResult<CosmosQuery> {
    let mut sql = String::from("SELECT * FROM c");
    let mut parameters = Vec::new();

    if !filters.is_empty() {
        let mut parts = Vec::with_capacity(filters.len());
        for filter in filters {
            parts.push(render_filter(table, filter, &mut parameters)?);
        }
        sql.push_str(" WHERE ");
        sql.push_str(&parts.join(" AND "));
    }

    Ok(CosmosQuery { sql, parameters })
}

fn render_filter(
    table: &Table,
    filter: &TableFilterClause,
    parameters: &mut Vec<Value>,
) -> CellarResult<String> {
    let path = column_path(table, &filter.column)?;
    let json_col = is_json_column(table, &filter.column);
    // Text operators on JSON columns search the serialized document so nested
    // keys like Data.Customer.Email are findable via contains on `data`.
    let text_expr = if json_col {
        format!("ToString({path})")
    } else {
        path.clone()
    };

    match filter.operator {
        TableFilterOperator::IsNull => Ok(format!(
            "(NOT IS_DEFINED({path}) OR IS_NULL({path}))"
        )),
        TableFilterOperator::IsNotNull => Ok(format!(
            "(IS_DEFINED({path}) AND NOT IS_NULL({path}))"
        )),
        TableFilterOperator::Equals | TableFilterOperator::NotEquals => {
            let value = required_value(filter)?;
            let param = push_param(parameters, bind_value(table, &filter.column, value));
            let op = if filter.operator == TableFilterOperator::Equals {
                "="
            } else {
                "!="
            };
            if json_col {
                Ok(format!("{text_expr} {op} {param}"))
            } else {
                Ok(format!("{path} {op} {param}"))
            }
        }
        TableFilterOperator::Contains => {
            let value = required_value(filter)?;
            // Avoid the 3-arg CONTAINS(..., true) form — on cross-partition
            // containers the gateway treats it as a non-gateway query. UPPER on
            // both sides keeps case-insensitive match with a simple filter shape.
            let param = push_param(parameters, Value::String(value.to_ascii_uppercase()));
            Ok(format!("CONTAINS(UPPER({text_expr}), {param})"))
        }
        TableFilterOperator::NotContains => {
            let value = required_value(filter)?;
            let param = push_param(parameters, Value::String(value.to_ascii_uppercase()));
            Ok(format!("NOT CONTAINS(UPPER({text_expr}), {param})"))
        }
        TableFilterOperator::StartsWith => {
            let value = required_value(filter)?;
            let param = push_param(parameters, Value::String(value.to_ascii_uppercase()));
            Ok(format!("STARTSWITH(UPPER({text_expr}), {param})"))
        }
        TableFilterOperator::EndsWith => {
            let value = required_value(filter)?;
            let param = push_param(parameters, Value::String(value.to_ascii_uppercase()));
            Ok(format!("ENDSWITH(UPPER({text_expr}), {param})"))
        }
        TableFilterOperator::Like => {
            let value = required_value(filter)?;
            // Translate simple LIKE patterns; fall back to RegexMatch.
            if let Some(expr) = like_to_cosmos(&text_expr, value, parameters) {
                Ok(expr)
            } else {
                // RegexMatch + cross-partition often can't be gateway-served;
                // prefer a CONTAINS fallback on the literal body with % stripped.
                let stripped = value.replace('%', "").replace('_', "");
                if stripped.is_empty() {
                    Ok("true".into())
                } else {
                    let param =
                        push_param(parameters, Value::String(stripped.to_ascii_uppercase()));
                    Ok(format!("CONTAINS(UPPER({text_expr}), {param})"))
                }
            }
        }
        TableFilterOperator::GreaterThan
        | TableFilterOperator::GreaterThanOrEqual
        | TableFilterOperator::LessThan
        | TableFilterOperator::LessThanOrEqual => {
            let value = required_value(filter)?;
            let param = push_param(parameters, bind_value(table, &filter.column, value));
            let op = match filter.operator {
                TableFilterOperator::GreaterThan => ">",
                TableFilterOperator::GreaterThanOrEqual => ">=",
                TableFilterOperator::LessThan => "<",
                TableFilterOperator::LessThanOrEqual => "<=",
                _ => unreachable!(),
            };
            Ok(format!("{path} {op} {param}"))
        }
    }
}

fn required_value<'a>(filter: &'a TableFilterClause) -> CellarResult<&'a str> {
    filter
        .value
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            CellarError::invalid_config(format!(
                "filter on '{}' needs a value",
                filter.column
            ))
        })
}

fn push_param(parameters: &mut Vec<Value>, value: Value) -> String {
    let name = format!("@p{}", parameters.len());
    parameters.push(json!({ "name": name, "value": value }));
    name
}

fn bind_value(table: &Table, column: &str, raw: &str) -> Value {
    let data_type = table
        .columns
        .iter()
        .find(|c| c.name == column)
        .map(|c| c.data_type.as_str())
        .unwrap_or("string");
    match data_type {
        "integer" | "int" | "bigint" | "long" => raw
            .parse::<i64>()
            .map(Value::from)
            .unwrap_or_else(|_| Value::String(raw.to_string())),
        "double" | "float" | "number" => raw
            .parse::<f64>()
            .map(Value::from)
            .unwrap_or_else(|_| Value::String(raw.to_string())),
        "boolean" | "bool" => match raw.to_ascii_lowercase().as_str() {
            "true" | "1" => Value::Bool(true),
            "false" | "0" => Value::Bool(false),
            _ => Value::String(raw.to_string()),
        },
        _ => Value::String(raw.to_string()),
    }
}

fn is_json_column(table: &Table, column: &str) -> bool {
    table
        .columns
        .iter()
        .find(|c| c.name == column)
        .map(|c| {
            matches!(
                c.data_type.as_str(),
                "json" | "jsonb" | "object" | "array" | "map"
            )
        })
        .unwrap_or(false)
}

/// Resolve `column` or dotted `Data.Customer.Email` into `c["Data"]["Customer"]["Email"]`.
/// Top-level names must exist on the table; nested segments after a known root are allowed.
fn column_path(table: &Table, column: &str) -> CellarResult<String> {
    let segments: Vec<&str> = column.split('.').filter(|s| !s.is_empty()).collect();
    if segments.is_empty() {
        return Err(CellarError::invalid_config("filter column is empty"));
    }
    for segment in &segments {
        if !is_safe_ident(segment) {
            return Err(CellarError::invalid_config(format!(
                "invalid filter column segment '{segment}'"
            )));
        }
    }
    let root = segments[0];
    if !table.columns.iter().any(|c| c.name == root) {
        return Err(CellarError::invalid_config(format!(
            "unknown column '{root}'"
        )));
    }
    Ok(format!(
        "c{}",
        segments
            .iter()
            .map(|s| format!("[\"{}\"]", s.replace('"', "")))
            .collect::<String>()
    ))
}

fn is_safe_ident(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Map `foo%` / `%foo` / `%foo%` / `foo` to Cosmos string functions. Returns
/// `None` when the pattern needs a full regex translation.
fn like_to_cosmos(expr: &str, pattern: &str, parameters: &mut Vec<Value>) -> Option<String> {
    let has_underscore = pattern.contains('_');
    if has_underscore {
        return None;
    }
    let percent_count = pattern.matches('%').count();
    if percent_count > 2 {
        return None;
    }
    if pattern == "%" {
        return Some("true".into());
    }
    let upper_expr = format!("UPPER({expr})");
    if !pattern.contains('%') {
        let param = push_param(parameters, Value::String(pattern.to_ascii_uppercase()));
        return Some(format!("STRINGEQUALS({upper_expr}, {param})"));
    }
    if let Some(inner) = pattern.strip_prefix('%').and_then(|p| p.strip_suffix('%')) {
        if inner.contains('%') {
            return None;
        }
        let param = push_param(parameters, Value::String(inner.to_ascii_uppercase()));
        return Some(format!("CONTAINS({upper_expr}, {param})"));
    }
    if let Some(prefix) = pattern.strip_suffix('%') {
        if prefix.contains('%') {
            return None;
        }
        let param = push_param(parameters, Value::String(prefix.to_ascii_uppercase()));
        return Some(format!("STARTSWITH({upper_expr}, {param})"));
    }
    if let Some(suffix) = pattern.strip_prefix('%') {
        if suffix.contains('%') {
            return None;
        }
        let param = push_param(parameters, Value::String(suffix.to_ascii_uppercase()));
        return Some(format!("ENDSWITH({upper_expr}, {param})"));
    }
    None
}

fn sort_documents(documents: &mut [Map<String, Value>], sorts: &[TableSortClause]) {
    if sorts.is_empty() {
        return;
    }
    documents.sort_by(|left, right| {
        for sort in sorts {
            let lv = left.get(&sort.column).unwrap_or(&Value::Null);
            let rv = right.get(&sort.column).unwrap_or(&Value::Null);
            let cmp = compare_json_values(lv, rv);
            let cmp = match sort.direction {
                SortDirection::Asc => cmp,
                SortDirection::Desc => cmp.reverse(),
            };
            if cmp != std::cmp::Ordering::Equal {
                return cmp;
            }
        }
        std::cmp::Ordering::Equal
    });
}

fn compare_json_values(left: &Value, right: &Value) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match (left, right) {
        (Value::Null, Value::Null) => Ordering::Equal,
        (Value::Null, _) => Ordering::Less,
        (_, Value::Null) => Ordering::Greater,
        (Value::Number(a), Value::Number(b)) => a
            .as_f64()
            .partial_cmp(&b.as_f64())
            .unwrap_or(Ordering::Equal),
        (Value::String(a), Value::String(b)) => a.cmp(b),
        (Value::Bool(a), Value::Bool(b)) => a.cmp(b),
        _ => left.to_string().cmp(&right.to_string()),
    }
}

fn master_key_authorization(
    verb: &str,
    resource_type: &str,
    resource_link: &str,
    date: &str,
    key: &[u8],
) -> CellarResult<String> {
    let payload = format!(
        "{}\n{}\n{}\n{}\n\n",
        verb.to_lowercase(),
        resource_type.to_lowercase(),
        resource_link,
        date.to_lowercase()
    );
    let mut mac = HmacSha256::new_from_slice(key)
        .map_err(|e| CellarError::invalid_config(format!("invalid Cosmos key: {e}")))?;
    mac.update(payload.as_bytes());
    let signature = BASE64.encode(mac.finalize().into_bytes());
    let token = format!("type=master&ver=1.0&sig={signature}");
    Ok(urlencoding::encode(&token).into_owned())
}

fn rfc1123_now() -> String {
    // Prefer chrono-free formatting: HTTP-date via httpdate would be ideal, but
    // keep deps light and format from SystemTime.
    use std::time::Duration as StdDuration;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(StdDuration::ZERO)
        .as_secs();
    httpdate_from_unix(now)
}

fn httpdate_from_unix(secs: u64) -> String {
    // Days since Unix epoch → civil date, then RFC 1123.
    const DAYS: [&str; 7] = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"];
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let day_secs = secs % 86_400;
    let days = secs / 86_400;
    let weekday = DAYS[(days % 7) as usize];
    let (year, month, day) = civil_from_days(days as i64);
    let hour = day_secs / 3600;
    let minute = (day_secs % 3600) / 60;
    let second = day_secs % 60;
    format!("{weekday}, {day:02} {month} {year:04} {hour:02}:{minute:02}:{second:02} GMT", month = MONTHS[month as usize])
}

/// Convert days since Unix epoch to (year, month 0-11, day 1-31).
fn civil_from_days(days: i64) -> (i32, u32, u32) {
    // Algorithm from Howard Hinnant / civil_from_days.
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, (m - 1) as u32, d as u32)
}

fn infer_columns(documents: &[Map<String, Value>]) -> Vec<Column> {
    let mut fields: BTreeMap<String, BTreeSet<&'static str>> = BTreeMap::new();
    for doc in documents {
        for (name, value) in doc {
            if name == "id" || name.starts_with('_') {
                continue;
            }
            fields
                .entry(name.clone())
                .or_default()
                .insert(json_type_name(value));
        }
    }

    let mut columns = vec![column("id", "string", false, true, 1)];
    // Surface a few useful system properties when present.
    for (name, data_type, ordinal) in [
        ("_ts", "integer", 2u32),
        ("_etag", "string", 3),
        ("_rid", "string", 4),
    ] {
        if documents.iter().any(|d| d.contains_key(name)) {
            columns.push(column(name, data_type, true, false, ordinal));
        }
    }
    let start = columns.len() as u32 + 1;
    for (idx, (name, types)) in fields.into_iter().enumerate() {
        let data_type = if types.len() == 1 {
            types.into_iter().next().unwrap_or("unknown")
        } else {
            "mixed"
        };
        columns.push(column(&name, data_type, true, false, start + idx as u32));
    }
    columns
}

fn columns_for_browse(table: &Table, documents: &[Map<String, Value>]) -> Vec<Column> {
    let mut columns = table.columns.clone();
    let existing = columns
        .iter()
        .map(|c| c.name.as_str())
        .collect::<BTreeSet<_>>();
    let mut extra = BTreeMap::new();
    for doc in documents {
        for (name, value) in doc {
            if !existing.contains(name.as_str()) && !name.starts_with('_') {
                extra.entry(name.clone()).or_insert(json_type_name(value));
            }
        }
    }
    let start = columns.len() as u32 + 1;
    for (idx, (name, data_type)) in extra.into_iter().enumerate() {
        columns.push(column(&name, data_type, true, false, start + idx as u32));
    }
    columns
}

fn column(
    name: &str,
    data_type: &str,
    nullable: bool,
    is_primary_key: bool,
    ordinal: u32,
) -> Column {
    Column {
        name: name.into(),
        data_type: data_type.into(),
        nullable,
        default: None,
        is_primary_key,
        ordinal,
        comment: None,
    }
}

fn row_for_document(document: &Map<String, Value>, columns: &[Column]) -> Row {
    columns
        .iter()
        .map(|column| {
            document
                .get(&column.name)
                .map(to_cell_value)
                .unwrap_or(CellValue::Null)
        })
        .collect()
}

fn json_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(n) if n.is_i64() || n.is_u64() => "integer",
        Value::Number(_) => "double",
        Value::String(_) => "string",
        // Report nested documents/arrays as `json` so the desktop grid's rich
        // JSON renderer (tree + pretty copy) applies — same as Postgres jsonb.
        Value::Array(_) | Value::Object(_) => "json",
    }
}

fn to_cell_value(value: &Value) -> CellValue {
    match value {
        Value::Null => CellValue::Null,
        Value::Bool(v) => CellValue::Bool(*v),
        Value::Number(n) => n
            .as_i64()
            .map(CellValue::Int)
            .or_else(|| n.as_u64().map(|_| CellValue::Numeric(n.to_string())))
            .or_else(|| n.as_f64().map(CellValue::Float))
            .unwrap_or_else(|| CellValue::Numeric(n.to_string())),
        Value::String(v) => CellValue::Text(v.clone()),
        Value::Array(_) | Value::Object(_) => CellValue::Json(value.clone()),
    }
}

fn cosmos_base_url(config: &ConnectionConfig) -> CellarResult<Url> {
    let host = config.host.trim().trim_end_matches('/');
    // Accept either a bare hostname or a full https://… endpoint from the portal.
    let host = host
        .strip_prefix("https://")
        .or_else(|| host.strip_prefix("http://"))
        .unwrap_or(host)
        .split('/')
        .next()
        .unwrap_or(host);
    let scheme = if config.ssl_mode == SslMode::Disable {
        "http"
    } else {
        "https"
    };
    let default_port = (scheme == "https" && (config.port == 443 || config.port == 0))
        || (scheme == "http" && (config.port == 80 || config.port == 0));
    let base = if default_port {
        format!("{scheme}://{host}")
    } else {
        format!("{scheme}://{host}:{}", config.port)
    };
    Url::parse(&base).map_err(|e| CellarError::invalid_config(e.to_string()))
}

fn account_name(config: &ConnectionConfig) -> String {
    let host = config
        .host
        .trim()
        .trim_end_matches('/')
        .strip_prefix("https://")
        .or_else(|| {
            config
                .host
                .trim()
                .strip_prefix("http://")
        })
        .unwrap_or(config.host.trim());
    host.split('.')
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or("cosmos")
        .into()
}

fn user_agent(config: &ConnectionConfig) -> String {
    config
        .application_name
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("cellar")
        .to_string()
}

fn map_cosmos_status(status: u16, body: String) -> CellarError {
    let message = cosmos_error_message(&body).unwrap_or(body);
    // Gateway-only REST clients can't fan out ORDER BY / aggregates across
    // partitions. Surface a shorter explanation when Cosmos says so.
    let message = if message.contains("can not be directly served by the gateway")
        || message.contains("cannot be directly served by the gateway")
    {
        "This Cosmos query can't run as a cross-partition gateway query. \
Try filtering without Order by, or filter on a partition-key column. \
(Full partition fan-out is not implemented in Cellar yet.)"
            .to_string()
    } else {
        message
    };
    match status {
        401 | 403 => CellarError::Authentication(message),
        404 => CellarError::connection(message),
        408 | 429 | 500..=599 => CellarError::Timeout(message),
        _ => CellarError::connection(message),
    }
}

fn cosmos_error_message(body: &str) -> Option<String> {
    let value: Value = serde_json::from_str(body).ok()?;
    value
        .get("message")
        .or_else(|| value.get("Message"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn as_cosmos<'a>(conn: &'a dyn Connection) -> CellarResult<&'a CosmosConnection> {
    conn.as_any()
        .downcast_ref::<CosmosConnection>()
        .ok_or_else(|| {
            CellarError::NotConnected(format!(
                "expected cosmos connection, got {}",
                conn.info().engine.as_str()
            ))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn config() -> ConnectionConfig {
        ConnectionConfig {
            id: "cosmos".into(),
            name: "Cosmos".into(),
            engine: Engine::Cosmos,
            host: "myaccount.documents.azure.com".into(),
            port: 443,
            database: "".into(),
            user: "".into(),
            ssl_mode: SslMode::Require,
            env_tag: None,
            application_name: Some("cellar-test".into()),
            color: None,
        }
    }

    #[test]
    fn builds_account_base_url_from_hostname_or_portal_uri() {
        let url = cosmos_base_url(&config()).expect("base url");
        assert_eq!(url.as_str(), "https://myaccount.documents.azure.com/");
        assert_eq!(account_name(&config()), "myaccount");

        let mut with_scheme = config();
        with_scheme.host = "https://myaccount.documents.azure.com:443/".into();
        let url = cosmos_base_url(&with_scheme).expect("base url");
        assert_eq!(url.as_str(), "https://myaccount.documents.azure.com/");
        assert_eq!(account_name(&with_scheme), "myaccount");
    }

    #[test]
    fn authorization_signature_is_stable_for_known_payload() {
        // 64 zero bytes — valid Cosmo key material length for HMAC-SHA256.
        let key = [0u8; 64];
        let date = "Tue, 01 Jan 2019 00:00:00 GMT";
        let auth = master_key_authorization("GET", "dbs", "", date, &key).expect("auth");
        assert!(auth.starts_with("type%3Dmaster%26ver%3D1.0%26sig%3D"));
        // Recomputing must be deterministic.
        let auth2 = master_key_authorization("GET", "dbs", "", date, &key).expect("auth");
        assert_eq!(auth, auth2);

        // Document feed auth uses the container resource link, not empty.
        let docs = master_key_authorization(
            "POST",
            "docs",
            "dbs/mydb/colls/mycoll",
            date,
            &key,
        )
        .expect("docs auth");
        assert_ne!(auth, docs);
    }

    #[test]
    fn httpdate_formats_unix_epoch_thursday() {
        assert_eq!(httpdate_from_unix(0), "Thu, 01 Jan 1970 00:00:00 GMT");
        assert_eq!(
            httpdate_from_unix(1_546_300_800),
            "Tue, 01 Jan 2019 00:00:00 GMT"
        );
    }

    #[test]
    fn infers_columns_and_maps_rows_from_documents() {
        let documents = vec![
            json!({
                "id": "ada",
                "_ts": 1_700_000_000,
                "_etag": "\"0000\"",
                "active": true,
                "age": 37,
                "name": "Ada",
                "prefs": { "theme": "dark" }
            })
            .as_object()
            .cloned()
            .unwrap(),
        ];

        let columns = infer_columns(&documents);
        assert!(columns.iter().any(|c| c.name == "id" && c.is_primary_key));
        assert!(columns
            .iter()
            .any(|c| c.name == "active" && c.data_type == "boolean"));
        assert!(columns
            .iter()
            .any(|c| c.name == "age" && c.data_type == "integer"));
        assert!(columns.iter().any(|c| c.name == "_ts"));

        let row = row_for_document(&documents[0], &columns);
        assert_eq!(row[0], CellValue::Text("ada".into()));
        let name_pos = columns.iter().position(|c| c.name == "name").unwrap();
        assert_eq!(row[name_pos], CellValue::Text("Ada".into()));
        let prefs_pos = columns.iter().position(|c| c.name == "prefs").unwrap();
        assert_eq!(row[prefs_pos], CellValue::Json(json!({ "theme": "dark" })));
    }

    #[test]
    fn builds_contains_filter_on_json_column_via_tostring() {
        let table = Table {
            name: "ibis".into(),
            schema: SCHEMA_NAME.into(),
            row_count: None,
            columns: vec![
                column("id", "string", false, true, 1),
                column("data", "json", true, false, 2),
                column("IbisServerCode", "string", true, false, 3),
            ],
            primary_key: vec!["id".into()],
            foreign_keys: vec![],
            indexes: vec![],
        };
        let query = build_browse_query(
            &table,
            &[TableFilterClause {
                column: "data".into(),
                operator: TableFilterOperator::Contains,
                value: Some("ajhackett".into()),
            }],
            &[],
        )
        .expect("query");
        assert_eq!(
            query.sql,
            "SELECT * FROM c WHERE CONTAINS(UPPER(ToString(c[\"data\"])), @p0)"
        );
        assert_eq!(query.parameters[0]["value"], json!("AJHACKETT"));
    }

    #[test]
    fn builds_equals_and_nested_path_filters() {
        let table = Table {
            name: "ibis".into(),
            schema: SCHEMA_NAME.into(),
            row_count: None,
            columns: vec![
                column("id", "string", false, true, 1),
                column("data", "json", true, false, 2),
                column("IbisServerCode", "string", true, false, 3),
            ],
            primary_key: vec!["id".into()],
            foreign_keys: vec![],
            indexes: vec![],
        };
        let query = build_browse_query(
            &table,
            &[
                TableFilterClause {
                    column: "IbisServerCode".into(),
                    operator: TableFilterOperator::Equals,
                    value: Some("skypark".into()),
                },
                TableFilterClause {
                    column: "data.Customer".into(),
                    operator: TableFilterOperator::IsNotNull,
                    value: None,
                },
            ],
            // Sorts are accepted but not emitted — gateway can't ORDER BY
            // across partitions over REST.
            &[TableSortClause {
                column: "id".into(),
                direction: SortDirection::Desc,
            }],
        )
        .expect("query");
        assert_eq!(
            query.sql,
            "SELECT * FROM c WHERE c[\"IbisServerCode\"] = @p0 AND (IS_DEFINED(c[\"data\"][\"Customer\"]) AND NOT IS_NULL(c[\"data\"][\"Customer\"]))"
        );
        assert!(!query.sql.contains("ORDER BY"));
    }

    #[test]
    fn rejects_unknown_filter_columns() {
        let table = Table {
            name: "ibis".into(),
            schema: SCHEMA_NAME.into(),
            row_count: None,
            columns: vec![column("id", "string", false, true, 1)],
            primary_key: vec!["id".into()],
            foreign_keys: vec![],
            indexes: vec![],
        };
        let err = build_browse_query(
            &table,
            &[TableFilterClause {
                column: "nope".into(),
                operator: TableFilterOperator::Equals,
                value: Some("x".into()),
            }],
            &[],
        )
        .expect_err("unknown column");
        assert!(err.to_string().contains("unknown column"));
    }

    #[test]
    fn preserves_large_unsigned_integers() {
        let value = json!(u64::MAX);
        assert_eq!(
            to_cell_value(&value),
            CellValue::Numeric(u64::MAX.to_string())
        );
        assert_eq!(to_cell_value(&json!(42)), CellValue::Int(42));
        assert_eq!(to_cell_value(&json!(1.5)), CellValue::Float(1.5));
    }
}
