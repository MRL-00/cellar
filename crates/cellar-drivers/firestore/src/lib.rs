//! Firestore driver for Cellar, backed by the Google Cloud Firestore REST API.
//!
//! This first slice treats root collections as read-only tables under a
//! `documents` schema. It infers columns from a small document sample and maps
//! document values into Cellar's existing grid result contract.

use std::any::Any;
use std::collections::{BTreeMap, BTreeSet};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use cellar_core::driver::{Connection, ConnectionConfig, Driver, DriverInfo, Engine, SslMode};
use cellar_core::error::{CellarError, CellarResult};
use cellar_core::query::{
    NoticeCapture, PlanMode, Query, QueryPlan, QueryResult, TableBrowseRequest,
};
use cellar_core::schema::{Column, Database, Index, Schema, Table};
use cellar_core::value::{CellValue, ColumnMeta, Row};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use reqwest::{Client, Method, Url};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const SCHEMA_NAME: &str = "documents";
const DEFAULT_DATABASE_ID: &str = "(default)";
const DEFAULT_SAMPLE_SIZE: u32 = 25;
const DEFAULT_BROWSE_LIMIT: u32 = 500;
const DATASTORE_SCOPE: &str = "https://www.googleapis.com/auth/datastore";

#[derive(Debug, Default, Clone, Copy)]
pub struct FirestoreDriver;

pub async fn browse_collection(
    conn: &dyn Connection,
    request: &TableBrowseRequest,
    table: &Table,
) -> CellarResult<QueryResult> {
    let fs = as_firestore(conn)?;
    fs.browse_collection(request, table).await
}

#[async_trait]
impl Driver for FirestoreDriver {
    fn engine(&self) -> Engine {
        Engine::Firestore
    }

    async fn connect(
        &self,
        config: &ConnectionConfig,
        secret: Option<&str>,
    ) -> CellarResult<Box<dyn Connection>> {
        let connection = FirestoreConnection::open(config, secret).await?;
        Ok(Box::new(connection))
    }

    async fn introspect(&self, conn: &dyn Connection) -> CellarResult<Vec<Database>> {
        let fs = as_firestore(conn)?;
        fs.introspect().await
    }

    async fn execute_query(
        &self,
        _conn: &dyn Connection,
        _query: &Query,
    ) -> CellarResult<QueryResult> {
        Err(CellarError::query(
            "Firestore SQL/query execution is not supported yet; browse root collections from the sidebar",
        ))
    }

    async fn explain_query(
        &self,
        _conn: &dyn Connection,
        _query: &Query,
        _mode: PlanMode,
    ) -> CellarResult<QueryPlan> {
        Err(CellarError::invalid_config(
            "Firestore does not expose SQL execution plans through Cellar yet",
        ))
    }
}

pub struct FirestoreConnection {
    info: DriverInfo,
    client: Client,
    base_url: Url,
    project_id: String,
    database_id: String,
    auth: AuthMode,
}

impl FirestoreConnection {
    async fn open(config: &ConnectionConfig, secret: Option<&str>) -> CellarResult<Self> {
        if config.database.trim().is_empty() {
            return Err(CellarError::invalid_config(
                "Firestore project ID is required",
            ));
        }
        let project_id = config.database.trim().to_string();
        let database_id = firestore_database_id(config);
        let client = Client::builder()
            .user_agent(user_agent(config))
            .build()
            .map_err(|e| CellarError::connection(e.to_string()))?;
        let base_url = firestore_base_url(config)?;
        let auth = AuthMode::from_secret(secret).await?;
        let connection = Self {
            info: DriverInfo {
                engine: Engine::Firestore,
                version: "Cloud Firestore REST v1".into(),
            },
            client,
            base_url,
            project_id,
            database_id,
            auth,
        };
        connection.list_collection_ids(None, 1).await?;
        Ok(connection)
    }

    async fn introspect(&self) -> CellarResult<Vec<Database>> {
        let collection_ids = self.list_collection_ids(None, 1000).await?;
        let mut tables = Vec::with_capacity(collection_ids.len());

        for collection_id in collection_ids {
            let documents = self
                .list_documents(&collection_id, DEFAULT_SAMPLE_SIZE)
                .await
                .unwrap_or_default();
            let columns = infer_columns(&documents);
            tables.push(Table {
                name: collection_id,
                schema: SCHEMA_NAME.into(),
                row_count: None,
                columns,
                primary_key: vec!["__name__".into()],
                foreign_keys: vec![],
                indexes: vec![Index {
                    name: "__name__".into(),
                    columns: vec!["__name__".into()],
                    unique: true,
                    primary: true,
                }],
            });
        }

        Ok(vec![Database {
            name: self.project_id.clone(),
            is_default: true,
            schemas: vec![Schema {
                name: SCHEMA_NAME.into(),
                tables,
                views: vec![],
            }],
        }])
    }

    async fn browse_collection(
        &self,
        request: &TableBrowseRequest,
        table: &Table,
    ) -> CellarResult<QueryResult> {
        if request.schema != SCHEMA_NAME {
            return Err(CellarError::invalid_config(format!(
                "Firestore collection schema must be {SCHEMA_NAME}"
            )));
        }
        if !request.sorts.is_empty() || !request.filters.is_empty() {
            return Err(CellarError::query(
                "Firestore server-side grid sorting and filtering are not supported yet",
            ));
        }

        let started = Instant::now();
        let limit = request.limit.unwrap_or(DEFAULT_BROWSE_LIMIT);
        let documents = self.list_documents(&request.table, limit).await?;
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
                "Firestore REST responses do not include database notice frames",
            ),
            rows_affected: None,
            duration_ms: started.elapsed().as_millis() as u64,
            truncated: documents.len() as u32 >= limit,
            total_rows: None,
        })
    }

    async fn list_collection_ids(
        &self,
        parent_document: Option<&str>,
        page_size: u32,
    ) -> CellarResult<Vec<String>> {
        let mut url = self.documents_url(parent_document, Some("listCollectionIds"))?;
        let mut out = Vec::new();
        let mut page_token: Option<String> = None;

        loop {
            let mut body = json!({ "pageSize": page_size });
            if let Some(token) = page_token.as_deref() {
                body["pageToken"] = json!(token);
            }
            let response: ListCollectionIdsResponse = self
                .send_json(Method::POST, url.clone(), Some(body))
                .await?;
            out.extend(response.collection_ids);
            page_token = response.next_page_token;
            if page_token.as_deref().unwrap_or("").is_empty() {
                break;
            }
            url = self.documents_url(parent_document, Some("listCollectionIds"))?;
        }

        Ok(out)
    }

    async fn list_documents(
        &self,
        collection_id: &str,
        page_size: u32,
    ) -> CellarResult<Vec<Document>> {
        let mut url = self.documents_url(Some(collection_id), None)?;
        url.query_pairs_mut()
            .append_pair("pageSize", &page_size.to_string());
        let response: ListDocumentsResponse = self.send_json(Method::GET, url, None).await?;
        Ok(response.documents)
    }

    fn documents_url(
        &self,
        child_path: Option<&str>,
        rpc_suffix: Option<&str>,
    ) -> CellarResult<Url> {
        let mut url = self.base_url.clone();
        {
            let mut segments = url
                .path_segments_mut()
                .map_err(|_| CellarError::invalid_config("invalid Firestore API base URL"))?;
            segments.clear();
            segments.extend([
                "v1",
                "projects",
                &self.project_id,
                "databases",
                &self.database_id,
            ]);
            if let Some(path) = child_path {
                segments.push("documents");
                for part in path.split('/').filter(|p| !p.is_empty()) {
                    segments.push(part);
                }
            } else {
                segments.push("documents");
            }
        }
        if let Some(suffix) = rpc_suffix {
            let path = url.path().trim_end_matches('/').to_string();
            url.set_path(&format!("{path}:{suffix}"));
        }
        Ok(url)
    }

    async fn send_json<T: for<'de> Deserialize<'de>>(
        &self,
        method: Method,
        url: Url,
        body: Option<Value>,
    ) -> CellarResult<T> {
        let mut request = self.client.request(method, url);
        if let Some(token) = self.auth.bearer_token().await? {
            request = request.bearer_auth(token);
        }
        if let Some(body) = body {
            request = request.json(&body);
        }
        let response = request
            .send()
            .await
            .map_err(|e| CellarError::connection(e.to_string()))?;
        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(map_firestore_status(status.as_u16(), text));
        }
        response
            .json::<T>()
            .await
            .map_err(|e| CellarError::decode(e.to_string()))
    }
}

#[async_trait]
impl Connection for FirestoreConnection {
    fn info(&self) -> &DriverInfo {
        &self.info
    }

    async fn ping(&self) -> CellarResult<()> {
        self.list_collection_ids(None, 1).await.map(|_| ())
    }

    async fn close(&self) -> CellarResult<()> {
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Clone)]
enum AuthMode {
    None,
    Bearer(String),
    ServiceAccount(ServiceAccountKey),
}

impl AuthMode {
    async fn from_secret(secret: Option<&str>) -> CellarResult<Self> {
        let Some(secret) = secret.map(str::trim).filter(|s| !s.is_empty()) else {
            return Ok(Self::None);
        };
        if secret.starts_with('{') {
            let key: ServiceAccountKey = serde_json::from_str(secret).map_err(|e| {
                CellarError::invalid_config(format!("invalid service account JSON: {e}"))
            })?;
            if key.client_email.trim().is_empty() || key.private_key.trim().is_empty() {
                return Err(CellarError::invalid_config(
                    "service account JSON must include client_email and private_key",
                ));
            }
            return Ok(Self::ServiceAccount(key));
        }
        Ok(Self::Bearer(secret.to_string()))
    }

    async fn bearer_token(&self) -> CellarResult<Option<String>> {
        match self {
            Self::None => Ok(None),
            Self::Bearer(token) => Ok(Some(token.clone())),
            Self::ServiceAccount(key) => mint_service_account_token(key).await.map(Some),
        }
    }
}

#[derive(Clone, Deserialize)]
struct ServiceAccountKey {
    client_email: String,
    private_key: String,
    #[serde(default = "default_token_uri")]
    token_uri: String,
}

#[derive(Serialize)]
struct JwtClaims<'a> {
    iss: &'a str,
    scope: &'a str,
    aud: &'a str,
    iat: usize,
    exp: usize,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

async fn mint_service_account_token(key: &ServiceAccountKey) -> CellarResult<String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| CellarError::Internal(e.to_string()))?
        .as_secs() as usize;
    let claims = JwtClaims {
        iss: &key.client_email,
        scope: DATASTORE_SCOPE,
        aud: &key.token_uri,
        iat: now,
        exp: now + 3600,
    };
    let assertion = encode(
        &Header::new(Algorithm::RS256),
        &claims,
        &EncodingKey::from_rsa_pem(key.private_key.as_bytes())
            .map_err(|e| CellarError::Authentication(e.to_string()))?,
    )
    .map_err(|e| CellarError::Authentication(e.to_string()))?;

    let client = Client::new();
    let response = client
        .post(&key.token_uri)
        .form(&[
            ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
            ("assertion", assertion.as_str()),
        ])
        .send()
        .await
        .map_err(|e| CellarError::Authentication(e.to_string()))?;
    let status = response.status();
    if !status.is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err(CellarError::Authentication(text));
    }
    let token = response
        .json::<TokenResponse>()
        .await
        .map_err(|e| CellarError::Authentication(e.to_string()))?;
    Ok(token.access_token)
}

fn default_token_uri() -> String {
    "https://oauth2.googleapis.com/token".into()
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListCollectionIdsResponse {
    #[serde(default)]
    collection_ids: Vec<String>,
    next_page_token: Option<String>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListDocumentsResponse {
    #[serde(default)]
    documents: Vec<Document>,
}

#[derive(Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Document {
    name: String,
    #[serde(default)]
    fields: BTreeMap<String, FirestoreValue>,
    create_time: Option<String>,
    update_time: Option<String>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
enum FirestoreValue {
    NullValue(Value),
    BooleanValue(bool),
    IntegerValue(String),
    DoubleValue(f64),
    TimestampValue(String),
    StringValue(String),
    BytesValue(String),
    ReferenceValue(String),
    GeoPointValue(Value),
    ArrayValue(Value),
    MapValue(Value),
}

fn infer_columns(documents: &[Document]) -> Vec<Column> {
    let mut fields: BTreeMap<String, BTreeSet<&'static str>> = BTreeMap::new();
    for doc in documents {
        for (name, value) in &doc.fields {
            fields
                .entry(name.clone())
                .or_default()
                .insert(value.type_name());
        }
    }

    let mut columns = vec![
        column("__name__", "string", false, true, 1),
        column("__create_time__", "timestamp", true, false, 2),
        column("__update_time__", "timestamp", true, false, 3),
    ];
    for (idx, (name, types)) in fields.into_iter().enumerate() {
        let data_type = if types.len() == 1 {
            types.into_iter().next().unwrap_or("unknown")
        } else {
            "mixed"
        };
        columns.push(column(&name, data_type, true, false, idx as u32 + 4));
    }
    columns
}

fn columns_for_browse(table: &Table, documents: &[Document]) -> Vec<Column> {
    let mut columns = table.columns.clone();
    let existing = columns
        .iter()
        .map(|c| c.name.as_str())
        .collect::<BTreeSet<_>>();
    let mut extra = BTreeMap::new();
    for doc in documents {
        for (name, value) in &doc.fields {
            if !existing.contains(name.as_str()) {
                extra.entry(name.clone()).or_insert(value.type_name());
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

fn row_for_document(document: &Document, columns: &[Column]) -> Row {
    columns
        .iter()
        .map(|column| match column.name.as_str() {
            "__name__" => CellValue::Text(short_document_name(&document.name).to_string()),
            "__create_time__" => optional_text(document.create_time.as_deref()),
            "__update_time__" => optional_text(document.update_time.as_deref()),
            other => document
                .fields
                .get(other)
                .map(FirestoreValue::to_cell_value)
                .unwrap_or(CellValue::Null),
        })
        .collect()
}

impl FirestoreValue {
    fn type_name(&self) -> &'static str {
        match self {
            Self::NullValue(_) => "null",
            Self::BooleanValue(_) => "boolean",
            Self::IntegerValue(_) => "integer",
            Self::DoubleValue(_) => "double",
            Self::TimestampValue(_) => "timestamp",
            Self::StringValue(_) => "string",
            Self::BytesValue(_) => "bytes",
            Self::ReferenceValue(_) => "reference",
            Self::GeoPointValue(_) => "geopoint",
            Self::ArrayValue(_) => "array",
            Self::MapValue(_) => "map",
        }
    }

    fn to_cell_value(&self) -> CellValue {
        match self {
            Self::NullValue(_) => CellValue::Null,
            Self::BooleanValue(v) => CellValue::Bool(*v),
            Self::IntegerValue(v) => v
                .parse::<i64>()
                .map(CellValue::Int)
                .unwrap_or_else(|_| CellValue::Numeric(v.clone())),
            Self::DoubleValue(v) => CellValue::Float(*v),
            Self::TimestampValue(v)
            | Self::StringValue(v)
            | Self::BytesValue(v)
            | Self::ReferenceValue(v) => CellValue::Text(v.clone()),
            Self::GeoPointValue(v) | Self::ArrayValue(v) | Self::MapValue(v) => {
                CellValue::Json(v.clone())
            }
        }
    }
}

fn optional_text(value: Option<&str>) -> CellValue {
    value
        .map(|v| CellValue::Text(v.to_string()))
        .unwrap_or(CellValue::Null)
}

fn short_document_name(name: &str) -> &str {
    name.rsplit('/').next().unwrap_or(name)
}

fn firestore_base_url(config: &ConnectionConfig) -> CellarResult<Url> {
    let host = if config.host.trim().is_empty() {
        "firestore.googleapis.com"
    } else {
        config.host.trim()
    };
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

fn firestore_database_id(config: &ConnectionConfig) -> String {
    let value = config.user.trim();
    if value.is_empty() {
        DEFAULT_DATABASE_ID.into()
    } else {
        value.into()
    }
}

fn user_agent(config: &ConnectionConfig) -> String {
    config
        .application_name
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("cellar")
        .to_string()
}

fn map_firestore_status(status: u16, body: String) -> CellarError {
    let message = firestore_error_message(&body).unwrap_or(body);
    match status {
        401 | 403 => CellarError::Authentication(message),
        408 | 429 | 500..=599 => CellarError::Timeout(message),
        _ => CellarError::connection(message),
    }
}

fn firestore_error_message(body: &str) -> Option<String> {
    let value: Value = serde_json::from_str(body).ok()?;
    value
        .get("error")
        .and_then(|e| e.get("message"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn as_firestore<'a>(conn: &'a dyn Connection) -> CellarResult<&'a FirestoreConnection> {
    conn.as_any()
        .downcast_ref::<FirestoreConnection>()
        .ok_or_else(|| {
            CellarError::NotConnected(format!(
                "expected firestore connection, got {}",
                conn.info().engine.as_str()
            ))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> ConnectionConfig {
        ConnectionConfig {
            id: "fs".into(),
            name: "Firestore".into(),
            engine: Engine::Firestore,
            host: "localhost".into(),
            port: 8080,
            database: "demo-project".into(),
            user: "".into(),
            ssl_mode: SslMode::Disable,
            env_tag: None,
            application_name: Some("cellar-test".into()),
            color: None,
        }
    }

    #[test]
    fn builds_emulator_base_url() {
        let url = firestore_base_url(&config()).expect("base url");
        assert_eq!(url.as_str(), "http://localhost:8080/");
    }

    #[test]
    fn infers_columns_and_maps_rows_from_documents() {
        let document: Document = serde_json::from_value(json!({
            "name": "projects/demo/databases/(default)/documents/users/ada",
            "createTime": "2026-01-01T00:00:00Z",
            "updateTime": "2026-01-02T00:00:00Z",
            "fields": {
                "active": { "booleanValue": true },
                "age": { "integerValue": "37" },
                "name": { "stringValue": "Ada" },
                "prefs": { "mapValue": { "fields": { "theme": { "stringValue": "dark" } } } }
            }
        }))
        .expect("document");

        let columns = infer_columns(std::slice::from_ref(&document));
        assert!(columns
            .iter()
            .any(|c| c.name == "__name__" && c.is_primary_key));
        assert!(columns
            .iter()
            .any(|c| c.name == "active" && c.data_type == "boolean"));
        assert!(columns
            .iter()
            .any(|c| c.name == "age" && c.data_type == "integer"));

        let row = row_for_document(&document, &columns);
        assert_eq!(row[0], CellValue::Text("ada".into()));
        let name_pos = columns.iter().position(|c| c.name == "name").unwrap();
        assert_eq!(row[name_pos], CellValue::Text("Ada".into()));
    }
}
