//! Cosmos REST connection: auth'd requests, resource feeds, and document browse.

use std::any::Any;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use cellar_core::driver::{Connection, ConnectionConfig, DriverInfo, Engine};
use cellar_core::error::{CellarError, CellarResult};
use cellar_core::query::{
    NoticeCapture, QueryResult, TableBrowseRequest,
};
use cellar_core::schema::{Database, Index, Schema, Table};
use cellar_core::value::ColumnMeta;
use reqwest::{Client, Method, RequestBuilder, Response, Url};
use serde::Deserialize;
use serde_json::{json, Map, Value};

use crate::auth::{master_key_authorization, rfc1123_now};
use crate::mapping::{columns_for_browse, infer_columns, row_for_document};
use crate::query::{build_browse_query, build_count_query, sort_documents, CosmosQuery};
use crate::{API_VERSION, DEFAULT_BROWSE_LIMIT, DEFAULT_SAMPLE_SIZE, REQUEST_TIMEOUT, SCHEMA_NAME};

pub struct CosmosConnection {
    info: DriverInfo,
    client: Client,
    base_url: Url,
    /// When set, introspection is scoped to this Cosmos database.
    scoped_database: Option<String>,
    master_key: Vec<u8>,
}

impl CosmosConnection {
    pub(crate) async fn open(config: &ConnectionConfig, secret: Option<&str>) -> CellarResult<Self> {
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
        connection.list_database_ids().await?;
        Ok(connection)
    }

    pub(crate) async fn introspect(&self) -> CellarResult<Vec<Database>> {
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

    pub(crate) async fn browse_table(
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
        sort_documents(&mut documents, &request.sorts);
        let columns = columns_for_browse(table, &documents);
        let rows = documents
            .iter()
            .map(|doc| row_for_document(doc, &columns))
            .collect::<Vec<_>>();

        let total_rows = if request.include_total {
            match self
                .count_documents(database_id, &request.table, table, &request.filters)
                .await
            {
                Ok(n) => Some(n),
                // Cross-partition COUNT often can't be gateway-served; keep browse
                // usable and omit the total rather than failing the page load.
                Err(e) if is_gateway_limitation(&e) => None,
                Err(e) => return Err(e),
            }
        } else {
            None
        };

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
            total_rows,
        })
    }

    async fn list_database_ids(&self) -> CellarResult<Vec<String>> {
        if let Some(scoped) = &self.scoped_database {
            let _ = self.get_database(scoped).await?;
            return Ok(vec![scoped.clone()]);
        }
        let mut ids = Vec::new();
        let mut continuation: Option<String> = None;
        loop {
            let response = self
                .send(
                    Method::GET,
                    self.dbs_url()?,
                    "dbs",
                    "",
                    continuation.as_deref(),
                    &[],
                    RequestBody::None,
                )
                .await?;
            continuation = continuation_token(&response);
            let feed: DatabasesFeed = response
                .json()
                .await
                .map_err(|e| CellarError::decode(e.to_string()))?;
            ids.extend(feed.databases.into_iter().map(|d| d.id));
            if continuation.is_none() {
                break;
            }
        }
        Ok(ids)
    }

    async fn get_database(&self, database_id: &str) -> CellarResult<NamedResource> {
        let response = self
            .send(
                Method::GET,
                self.db_url(database_id)?,
                "dbs",
                format!("dbs/{database_id}"),
                None,
                &[],
                RequestBody::None,
            )
            .await?;
        response
            .json()
            .await
            .map_err(|e| CellarError::decode(e.to_string()))
    }

    async fn list_containers(&self, database_id: &str) -> CellarResult<Vec<String>> {
        let mut ids = Vec::new();
        let mut continuation: Option<String> = None;
        loop {
            let response = self
                .send(
                    Method::GET,
                    self.colls_url(database_id)?,
                    "colls",
                    format!("dbs/{database_id}"),
                    continuation.as_deref(),
                    &[],
                    RequestBody::None,
                )
                .await?;
            continuation = continuation_token(&response);
            let feed: CollectionsFeed = response
                .json()
                .await
                .map_err(|e| CellarError::decode(e.to_string()))?;
            ids.extend(feed.collections.into_iter().map(|c| c.id));
            if continuation.is_none() {
                break;
            }
        }
        Ok(ids)
    }

    async fn count_documents(
        &self,
        database_id: &str,
        container_id: &str,
        table: &Table,
        filters: &[cellar_core::query::TableFilterClause],
    ) -> CellarResult<u64> {
        let query = build_count_query(table, filters)?;
        let page = self
            .query_documents_page(database_id, container_id, 1, None, &query)
            .await?;
        // SELECT VALUE COUNT(1) returns a single scalar in Documents.
        match page.documents.first() {
            Some(doc) => {
                // VALUE queries may deserialize as a bare number wrapped oddly;
                // also accept {"$1": n} style objects from some gateways.
                if let Some(n) = scalar_count(doc) {
                    return Ok(n);
                }
                Err(CellarError::decode(
                    "Cosmos COUNT response did not contain a numeric total",
                ))
            }
            None => Ok(0),
        }
    }

    async fn query_documents_page(
        &self,
        database_id: &str,
        container_id: &str,
        max_items: i32,
        continuation: Option<&str>,
        query: &CosmosQuery,
    ) -> CellarResult<DocumentsPage> {
        let extra = [
            ("x-ms-documentdb-isquery", "True".into()),
            ("x-ms-documentdb-query-enablecrosspartition", "True".into()),
            ("x-ms-max-item-count", max_items.to_string()),
            ("Content-Type", "application/query+json".into()),
        ];
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
                continuation,
                &extra,
                RequestBody::Json(body),
            )
            .await?;
        let next_continuation = continuation_token(&response);
        // COUNT / VALUE queries return Documents as a JSON array of scalars.
        // Try typed feed first; fall back to raw Value for scalar arrays.
        let body_bytes = response
            .bytes()
            .await
            .map_err(|e| CellarError::decode(e.to_string()))?;
        if let Ok(feed) = serde_json::from_slice::<DocumentsFeed>(&body_bytes) {
            return Ok(DocumentsPage {
                documents: feed.documents,
                continuation: next_continuation,
            });
        }
        let value: Value =
            serde_json::from_slice(&body_bytes).map_err(|e| CellarError::decode(e.to_string()))?;
        let documents = value
            .get("Documents")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| match item {
                        Value::Object(map) => Some(map.clone()),
                        Value::Number(n) => {
                            let mut map = Map::new();
                            map.insert("$1".into(), Value::Number(n.clone()));
                            Some(map)
                        }
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(DocumentsPage {
            documents,
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
        continuation: Option<&str>,
        extra_headers: &[(&str, String)],
        body: RequestBody,
    ) -> CellarResult<Response> {
        let deadline = Instant::now() + REQUEST_TIMEOUT;
        let resource_link = resource_link.as_ref().to_string();
        loop {
            let date = rfc1123_now();
            let authorization = master_key_authorization(
                method.as_str(),
                resource_type,
                &resource_link,
                &date,
                &self.master_key,
            )?;
            let mut request: RequestBuilder = self
                .client
                .request(method.clone(), url.clone())
                .header("Authorization", authorization)
                .header("x-ms-date", &date)
                .header("x-ms-version", API_VERSION)
                .header("Accept", "application/json");
            for (name, value) in extra_headers {
                request = request.header(*name, value);
            }
            if let Some(token) = continuation {
                request = request.header("x-ms-continuation", token);
            }
            request = match &body {
                RequestBody::None => request,
                RequestBody::Json(value) => request.json(value),
            };
            let response = request
                .send()
                .await
                .map_err(|e| CellarError::connection(e.to_string()))?;
            let status = response.status();
            if status.as_u16() == 429 {
                let retry_after_ms = response
                    .headers()
                    .get("x-ms-retry-after-ms")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(1_000);
                let wait = Duration::from_millis(retry_after_ms);
                if Instant::now() + wait >= deadline {
                    let text = response.text().await.unwrap_or_default();
                    return Err(map_cosmos_status(429, text));
                }
                // Drain the body so the connection can be reused, then wait.
                let _ = response.bytes().await;
                tokio::time::sleep(wait).await;
                continue;
            }
            if !status.is_success() {
                let text = response.text().await.unwrap_or_default();
                return Err(map_cosmos_status(status.as_u16(), text));
            }
            return Ok(response);
        }
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

fn continuation_token(response: &Response) -> Option<String> {
    response
        .headers()
        .get("x-ms-continuation")
        .and_then(|v| v.to_str().ok())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn scalar_count(doc: &Map<String, Value>) -> Option<u64> {
    if let Some(Value::Number(n)) = doc.get("$1") {
        return n.as_u64().or_else(|| n.as_i64().map(|i| i as u64));
    }
    // Single-field object with a number.
    if doc.len() == 1 {
        if let Some(Value::Number(n)) = doc.values().next() {
            return n.as_u64().or_else(|| n.as_i64().map(|i| i as u64));
        }
    }
    None
}

fn is_gateway_limitation(err: &CellarError) -> bool {
    let message = err.to_string();
    message.contains("cross-partition gateway query")
        || message.contains("can not be directly served by the gateway")
        || message.contains("cannot be directly served by the gateway")
}

/// Always HTTPS — Cosmos (including the emulator) requires TLS. Import the
/// emulator certificate into the trust store rather than sending the master
/// key over plaintext HTTP.
pub(crate) fn cosmos_base_url(config: &ConnectionConfig) -> CellarResult<Url> {
    let host = config.host.trim().trim_end_matches('/');
    let host = host
        .strip_prefix("https://")
        .or_else(|| host.strip_prefix("http://"))
        .unwrap_or(host)
        .split('/')
        .next()
        .unwrap_or(host);
    let default_port = config.port == 443 || config.port == 0;
    let base = if default_port {
        format!("https://{host}")
    } else {
        format!("https://{host}:{}", config.port)
    };
    Url::parse(&base).map_err(|e| CellarError::invalid_config(e.to_string()))
}

pub(crate) fn account_name(config: &ConnectionConfig) -> String {
    let host = config
        .host
        .trim()
        .trim_end_matches('/')
        .strip_prefix("https://")
        .or_else(|| config.host.trim().strip_prefix("http://"))
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

#[cfg(test)]
mod tests {
    use super::*;
    use cellar_core::driver::SslMode;

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
    fn cosmos_base_url_stays_https_when_ssl_disabled() {
        let mut cfg = config();
        cfg.ssl_mode = SslMode::Disable;
        cfg.host = "http://localhost".into();
        cfg.port = 8081;
        let url = cosmos_base_url(&cfg).expect("base url");
        assert_eq!(url.as_str(), "https://localhost:8081/");
        assert_eq!(url.scheme(), "https");
    }
}
