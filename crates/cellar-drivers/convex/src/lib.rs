//! Convex driver for Cellar, backed by the Convex streaming export HTTP API
//! (`/api/json_schemas`, `/api/list_snapshot`).
//!
//! This first slice treats Convex tables as read-only tables under a
//! `documents` schema, mirroring the Firestore driver. Columns are inferred
//! from a small document sample. Note: the streaming export API requires a
//! Convex Pro plan on the cloud service (it works against self-hosted and
//! local deployments regardless).

use std::any::Any;
use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use cellar_core::driver::{Connection, ConnectionConfig, Driver, DriverInfo, Engine, SslMode};
use cellar_core::error::{CellarError, CellarResult};
use cellar_core::query::{
    NoticeCapture, PlanMode, Query, QueryPlan, QueryResult, TableBrowseRequest,
};
use cellar_core::schema::{Column, Database, Index, Schema, Table};
use cellar_core::value::{CellValue, ColumnMeta, Row};
use reqwest::{Client, Url};
use serde::Deserialize;
use serde_json::{Map, Value};

const SCHEMA_NAME: &str = "documents";
const DEFAULT_SAMPLE_SIZE: usize = 25;
const DEFAULT_BROWSE_LIMIT: u32 = 500;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Default, Clone, Copy)]
pub struct ConvexDriver;

pub async fn browse_table(
    conn: &dyn Connection,
    request: &TableBrowseRequest,
    table: &Table,
) -> CellarResult<QueryResult> {
    let cx = as_convex(conn)?;
    cx.browse_table(request, table).await
}

#[async_trait]
impl Driver for ConvexDriver {
    fn engine(&self) -> Engine {
        Engine::Convex
    }

    async fn connect(
        &self,
        config: &ConnectionConfig,
        secret: Option<&str>,
    ) -> CellarResult<Box<dyn Connection>> {
        let connection = ConvexConnection::open(config, secret).await?;
        Ok(Box::new(connection))
    }

    async fn introspect(&self, conn: &dyn Connection) -> CellarResult<Vec<Database>> {
        let cx = as_convex(conn)?;
        cx.introspect().await
    }

    async fn execute_query(
        &self,
        _conn: &dyn Connection,
        _query: &Query,
    ) -> CellarResult<QueryResult> {
        Err(CellarError::query(
            "Convex SQL/query execution is not supported yet; browse tables from the sidebar",
        ))
    }

    async fn explain_query(
        &self,
        _conn: &dyn Connection,
        _query: &Query,
        _mode: PlanMode,
    ) -> CellarResult<QueryPlan> {
        Err(CellarError::invalid_config(
            "Convex does not expose SQL execution plans through Cellar yet",
        ))
    }
}

pub struct ConvexConnection {
    info: DriverInfo,
    client: Client,
    base_url: Url,
    deployment: String,
    deploy_key: Option<String>,
}

impl ConvexConnection {
    async fn open(config: &ConnectionConfig, secret: Option<&str>) -> CellarResult<Self> {
        if config.host.trim().is_empty() {
            return Err(CellarError::invalid_config(
                "Convex deployment URL is required, e.g. acoustic-panther-123.convex.cloud",
            ));
        }
        let client = Client::builder()
            .user_agent(user_agent(config))
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|e| CellarError::connection(e.to_string()))?;
        let base_url = convex_base_url(config)?;
        let deployment = deployment_name(config);
        let deploy_key = secret
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let connection = Self {
            info: DriverInfo {
                engine: Engine::Convex,
                version: "Convex streaming export API".into(),
            },
            client,
            base_url,
            deployment,
            deploy_key,
        };
        connection.table_names().await?;
        Ok(connection)
    }

    async fn introspect(&self) -> CellarResult<Vec<Database>> {
        let table_names = self.table_names().await?;
        let mut tables = Vec::with_capacity(table_names.len());

        for name in table_names {
            let documents = self.list_snapshot_page(&name, None, None).await?.values;
            let sample = &documents[..documents.len().min(DEFAULT_SAMPLE_SIZE)];
            let columns = infer_columns(sample);
            tables.push(Table {
                name,
                schema: SCHEMA_NAME.into(),
                row_count: None,
                columns,
                primary_key: vec!["_id".into()],
                foreign_keys: vec![],
                indexes: vec![Index {
                    name: "_id".into(),
                    columns: vec!["_id".into()],
                    unique: true,
                    primary: true,
                }],
            });
        }

        Ok(vec![Database {
            name: self.deployment.clone(),
            is_default: true,
            schemas: vec![Schema {
                name: SCHEMA_NAME.into(),
                tables,
                views: vec![],
            }],
        }])
    }

    async fn browse_table(
        &self,
        request: &TableBrowseRequest,
        table: &Table,
    ) -> CellarResult<QueryResult> {
        if request.schema != SCHEMA_NAME {
            return Err(CellarError::invalid_config(format!(
                "Convex table schema must be {SCHEMA_NAME}"
            )));
        }
        if !request.sorts.is_empty() || !request.filters.is_empty() {
            return Err(CellarError::query(
                "Convex server-side grid sorting and filtering are not supported yet",
            ));
        }

        let started = Instant::now();
        let limit = request.limit.unwrap_or(DEFAULT_BROWSE_LIMIT) as usize;
        // list_snapshot has no server-side offset, so paging re-reads from the
        // snapshot start and slices the requested window locally.
        let offset = request.offset.unwrap_or(0) as usize;
        let fetch_until = offset.saturating_add(limit);
        let mut documents: Vec<Map<String, Value>> = Vec::new();
        let mut snapshot: Option<i64> = None;
        let mut cursor: Option<String> = None;
        let mut truncated = false;

        loop {
            let page = self
                .list_snapshot_page(&request.table, snapshot, cursor.as_deref())
                .await?;
            snapshot = page.snapshot.or(snapshot);
            cursor = page.cursor;
            documents.extend(page.values);
            if documents.len() >= fetch_until {
                truncated = documents.len() > fetch_until || page.has_more;
                documents.truncate(fetch_until);
                break;
            }
            if !page.has_more {
                break;
            }
        }

        let start = offset.min(documents.len());
        let documents = documents.split_off(start);
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
                "Convex HTTP responses do not include database notice frames",
            ),
            rows_affected: None,
            duration_ms: started.elapsed().as_millis() as u64,
            truncated,
            total_rows: None,
        })
    }

    async fn table_names(&self) -> CellarResult<Vec<String>> {
        let mut url = self.api_url("json_schemas")?;
        url.query_pairs_mut().append_pair("format", "json");
        let schemas: Map<String, Value> = self.get_json(url).await?;
        Ok(schemas.into_iter().map(|(name, _)| name).collect())
    }

    async fn list_snapshot_page(
        &self,
        table_name: &str,
        snapshot: Option<i64>,
        cursor: Option<&str>,
    ) -> CellarResult<SnapshotPage> {
        let mut url = self.api_url("list_snapshot")?;
        {
            let mut pairs = url.query_pairs_mut();
            pairs.append_pair("tableName", table_name);
            pairs.append_pair("format", "json");
            if let Some(snapshot) = snapshot {
                pairs.append_pair("snapshot", &snapshot.to_string());
            }
            if let Some(cursor) = cursor {
                pairs.append_pair("cursor", cursor);
            }
        }
        self.get_json(url).await
    }

    fn api_url(&self, endpoint: &str) -> CellarResult<Url> {
        let mut url = self.base_url.clone();
        url.path_segments_mut()
            .map_err(|_| CellarError::invalid_config("invalid Convex deployment URL"))?
            .clear()
            .extend(["api", endpoint]);
        Ok(url)
    }

    async fn get_json<T: for<'de> Deserialize<'de>>(&self, url: Url) -> CellarResult<T> {
        let mut request = self.client.get(url);
        if let Some(key) = &self.deploy_key {
            request = request.header("Authorization", format!("Convex {key}"));
        }
        let response = request
            .send()
            .await
            .map_err(|e| CellarError::connection(e.to_string()))?;
        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(map_convex_status(status.as_u16(), text));
        }
        response
            .json::<T>()
            .await
            .map_err(|e| CellarError::decode(e.to_string()))
    }
}

#[async_trait]
impl Connection for ConvexConnection {
    fn info(&self) -> &DriverInfo {
        &self.info
    }

    async fn ping(&self) -> CellarResult<()> {
        self.table_names().await.map(|_| ())
    }

    async fn close(&self) -> CellarResult<()> {
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotPage {
    #[serde(default)]
    values: Vec<Map<String, Value>>,
    cursor: Option<String>,
    #[serde(default)]
    has_more: bool,
    snapshot: Option<i64>,
}

fn infer_columns(documents: &[Map<String, Value>]) -> Vec<Column> {
    let mut fields: BTreeMap<String, BTreeSet<&'static str>> = BTreeMap::new();
    for doc in documents {
        for (name, value) in doc {
            if name == "_id" || name == "_creationTime" {
                continue;
            }
            fields
                .entry(name.clone())
                .or_default()
                .insert(json_type_name(value));
        }
    }

    let mut columns = vec![
        column("_id", "id", false, true, 1),
        column("_creationTime", "timestamp", false, false, 2),
    ];
    for (idx, (name, types)) in fields.into_iter().enumerate() {
        let data_type = if types.len() == 1 {
            types.into_iter().next().unwrap_or("unknown")
        } else {
            "mixed"
        };
        columns.push(column(&name, data_type, true, false, idx as u32 + 3));
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
            if !existing.contains(name.as_str()) {
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
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn to_cell_value(value: &Value) -> CellValue {
    match value {
        Value::Null => CellValue::Null,
        Value::Bool(v) => CellValue::Bool(*v),
        // Check u64 before falling back to f64: integers above i64::MAX would
        // otherwise be rounded through the float path.
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

fn convex_base_url(config: &ConnectionConfig) -> CellarResult<Url> {
    let host = config.host.trim();
    let scheme = if config.ssl_mode == SslMode::Disable {
        "http"
    } else {
        "https"
    };
    let default_port = (scheme == "https" && config.port == 443)
        || (scheme == "http" && config.port == 80)
        || config.port == 0;
    let base = if default_port {
        format!("{scheme}://{host}")
    } else {
        format!("{scheme}://{host}:{}", config.port)
    };
    Url::parse(&base).map_err(|e| CellarError::invalid_config(e.to_string()))
}

fn deployment_name(config: &ConnectionConfig) -> String {
    let database = config.database.trim();
    if !database.is_empty() {
        return database.into();
    }
    // Fall back to the deployment slug from the host, e.g.
    // "acoustic-panther-123" from "acoustic-panther-123.convex.cloud".
    config
        .host
        .trim()
        .split('.')
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or("convex")
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

fn map_convex_status(status: u16, body: String) -> CellarError {
    let message = convex_error_message(&body).unwrap_or(body);
    match status {
        401 | 403 => CellarError::Authentication(message),
        408 | 429 | 500..=599 => CellarError::Timeout(message),
        _ => CellarError::connection(message),
    }
}

fn convex_error_message(body: &str) -> Option<String> {
    let value: Value = serde_json::from_str(body).ok()?;
    value
        .get("message")
        .or_else(|| value.get("error").and_then(|e| e.get("message")))
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn as_convex<'a>(conn: &'a dyn Connection) -> CellarResult<&'a ConvexConnection> {
    conn.as_any()
        .downcast_ref::<ConvexConnection>()
        .ok_or_else(|| {
            CellarError::NotConnected(format!(
                "expected convex connection, got {}",
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
            id: "cx".into(),
            name: "Convex".into(),
            engine: Engine::Convex,
            host: "acoustic-panther-123.convex.cloud".into(),
            port: 0,
            database: "".into(),
            user: "".into(),
            ssl_mode: SslMode::Require,
            env_tag: None,
            application_name: Some("cellar-test".into()),
            color: None,
        }
    }

    #[test]
    fn builds_cloud_base_url_and_deployment_name() {
        let url = convex_base_url(&config()).expect("base url");
        assert_eq!(url.as_str(), "https://acoustic-panther-123.convex.cloud/");
        assert_eq!(deployment_name(&config()), "acoustic-panther-123");
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

    #[test]
    fn infers_columns_and_maps_rows_from_documents() {
        let page: SnapshotPage = serde_json::from_value(json!({
            "values": [{
                "_id": "j57ab...",
                "_creationTime": 1_700_000_000_000.5,
                "active": true,
                "age": 37,
                "name": "Ada",
                "prefs": { "theme": "dark" }
            }],
            "cursor": null,
            "hasMore": false,
            "snapshot": 1234
        }))
        .expect("snapshot page");

        let columns = infer_columns(&page.values);
        assert!(columns.iter().any(|c| c.name == "_id" && c.is_primary_key));
        assert!(columns
            .iter()
            .any(|c| c.name == "active" && c.data_type == "boolean"));
        assert!(columns
            .iter()
            .any(|c| c.name == "age" && c.data_type == "integer"));

        let row = row_for_document(&page.values[0], &columns);
        assert_eq!(row[0], CellValue::Text("j57ab...".into()));
        let name_pos = columns.iter().position(|c| c.name == "name").unwrap();
        assert_eq!(row[name_pos], CellValue::Text("Ada".into()));
        let prefs_pos = columns.iter().position(|c| c.name == "prefs").unwrap();
        assert_eq!(row[prefs_pos], CellValue::Json(json!({ "theme": "dark" })));
    }
}
