use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use cellar_core::error::{CellarError, CellarResult};
use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::{QueryBuilder, Row, Sqlite, SqlitePool};
use tokio::fs;

const HISTORY_FILENAME: &str = "history.db";
const DEFAULT_LIMIT: u32 = 100;
const MAX_LIMIT: u32 = 500;
const MAX_ERROR_SUMMARY: usize = 500;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct QueryHistoryRecord {
    pub id: i64,
    pub connection_id: String,
    pub connection_name: Option<String>,
    pub tab_id: Option<String>,
    pub database: Option<String>,
    pub sql: String,
    pub executed_at_ms: i64,
    pub duration_ms: i64,
    pub success: bool,
    pub row_count: Option<i64>,
    pub truncated: bool,
    pub error_summary: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NewQueryHistoryRecord {
    pub connection_id: String,
    pub connection_name: Option<String>,
    pub tab_id: Option<String>,
    pub database: Option<String>,
    pub sql: String,
    pub duration_ms: i64,
    pub success: bool,
    pub row_count: Option<i64>,
    pub truncated: bool,
    pub error_summary: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct QueryHistoryFilter {
    pub connection_id: Option<String>,
    pub database: Option<String>,
    pub tab_id: Option<String>,
    pub search: Option<String>,
    pub limit: Option<u32>,
}

enum HistoryBackend {
    Sqlite(SqlitePool),
    Unavailable(String),
}

pub struct HistoryStore {
    backend: HistoryBackend,
}

impl HistoryStore {
    pub async fn load() -> CellarResult<Self> {
        let path = history_path()
            .ok_or_else(|| CellarError::invalid_config("could not resolve home directory"))?;
        Self::open_at(path).await
    }

    pub fn unavailable(detail: impl Into<String>) -> Self {
        Self {
            backend: HistoryBackend::Unavailable(detail.into()),
        }
    }

    pub async fn open_at(path: PathBuf) -> CellarResult<Self> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).await?;
        }
        secure_history_files(&path)?;
        let pool = connect_sqlite(&path).await?;
        migrate(&pool).await?;
        secure_history_files(&path)?;
        Ok(Self {
            backend: HistoryBackend::Sqlite(pool),
        })
    }

    pub async fn insert(&self, record: NewQueryHistoryRecord) -> CellarResult<()> {
        let pool = match self.pool() {
            Ok(pool) => pool,
            Err(_) => return Ok(()),
        };
        sqlx::query(
            r#"
            INSERT INTO query_history (
                connection_id,
                connection_name,
                tab_id,
                database_name,
                sql_text,
                executed_at_ms,
                duration_ms,
                success,
                row_count,
                truncated,
                error_summary
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(record.connection_id)
        .bind(record.connection_name.and_then(safe_connection_name))
        .bind(record.tab_id)
        .bind(record.database)
        .bind(redact_sql_for_storage(&record.sql))
        .bind(now_ms())
        .bind(record.duration_ms.max(0))
        .bind(if record.success { 1_i64 } else { 0_i64 })
        .bind(record.row_count)
        .bind(if record.truncated { 1_i64 } else { 0_i64 })
        .bind(record.error_summary.map(redact_error_summary))
        .execute(pool)
        .await
        .map_err(sqlx_error)?;
        Ok(())
    }

    pub async fn list(&self, filter: QueryHistoryFilter) -> CellarResult<Vec<QueryHistoryRecord>> {
        let pool = self.pool()?;
        let mut qb: QueryBuilder<'_, Sqlite> = QueryBuilder::new(
            r#"
            SELECT
                id,
                connection_id,
                connection_name,
                tab_id,
                database_name,
                sql_text,
                executed_at_ms,
                duration_ms,
                success,
                row_count,
                truncated,
                error_summary
            FROM query_history
            WHERE 1 = 1
            "#,
        );

        if let Some(connection_id) = non_empty(filter.connection_id) {
            qb.push(" AND connection_id = ");
            qb.push_bind(connection_id);
        }
        if let Some(database) = non_empty(filter.database) {
            qb.push(" AND database_name = ");
            qb.push_bind(database);
        }
        if let Some(tab_id) = non_empty(filter.tab_id) {
            qb.push(" AND tab_id = ");
            qb.push_bind(tab_id);
        }
        if let Some(search) = non_empty(filter.search) {
            let needle = format!("%{search}%");
            qb.push(" AND (sql_text LIKE ");
            qb.push_bind(needle.clone());
            qb.push(" OR error_summary LIKE ");
            qb.push_bind(needle);
            qb.push(")");
        }

        let limit = filter.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
        qb.push(" ORDER BY executed_at_ms DESC, id DESC LIMIT ");
        qb.push_bind(i64::from(limit));

        let rows = qb.build().fetch_all(pool).await.map_err(sqlx_error)?;
        rows.into_iter().map(record_from_row).collect()
    }

    fn pool(&self) -> CellarResult<&SqlitePool> {
        match &self.backend {
            HistoryBackend::Sqlite(pool) => Ok(pool),
            HistoryBackend::Unavailable(detail) => Err(CellarError::Internal(format!(
                "history unavailable: {detail}"
            ))),
        }
    }
}

#[cfg(unix)]
fn secure_history_files(path: &Path) -> CellarResult<()> {
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

    if !path.exists() {
        std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(path)?;
    }
    for path in [
        path.to_path_buf(),
        path_with_suffix(path, "-wal"),
        path_with_suffix(path, "-shm"),
    ] {
        if path.exists() {
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
        }
    }
    Ok(())
}

#[cfg(not(unix))]
fn secure_history_files(_: &Path) -> CellarResult<()> {
    Ok(())
}

#[cfg(unix)]
fn path_with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(suffix);
    value.into()
}

fn history_path() -> Option<PathBuf> {
    let mut p = crate::cellar_dir()?;
    p.push(HISTORY_FILENAME);
    Some(p)
}

async fn connect_sqlite(path: &Path) -> CellarResult<SqlitePool> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal);
    SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(options)
        .await
        .map_err(sqlx_error)
}

async fn migrate(pool: &SqlitePool) -> CellarResult<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS query_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            connection_id TEXT NOT NULL,
            connection_name TEXT,
            tab_id TEXT,
            database_name TEXT,
            sql_text TEXT NOT NULL,
            executed_at_ms INTEGER NOT NULL,
            duration_ms INTEGER NOT NULL,
            success INTEGER NOT NULL CHECK (success IN (0, 1)),
            row_count INTEGER,
            truncated INTEGER NOT NULL CHECK (truncated IN (0, 1)),
            error_summary TEXT
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(sqlx_error)?;

    for statement in [
        "CREATE INDEX IF NOT EXISTS idx_query_history_connection_time ON query_history(connection_id, executed_at_ms DESC)",
        "CREATE INDEX IF NOT EXISTS idx_query_history_tab_time ON query_history(tab_id, executed_at_ms DESC)",
        "CREATE INDEX IF NOT EXISTS idx_query_history_database_time ON query_history(database_name, executed_at_ms DESC)",
    ] {
        sqlx::query(statement)
            .execute(pool)
            .await
            .map_err(sqlx_error)?;
    }

    Ok(())
}

fn record_from_row(row: sqlx::sqlite::SqliteRow) -> CellarResult<QueryHistoryRecord> {
    Ok(QueryHistoryRecord {
        id: row.try_get("id").map_err(sqlx_error)?,
        connection_id: row.try_get("connection_id").map_err(sqlx_error)?,
        connection_name: row.try_get("connection_name").map_err(sqlx_error)?,
        tab_id: row.try_get("tab_id").map_err(sqlx_error)?,
        database: row.try_get("database_name").map_err(sqlx_error)?,
        sql: row.try_get("sql_text").map_err(sqlx_error)?,
        executed_at_ms: row.try_get("executed_at_ms").map_err(sqlx_error)?,
        duration_ms: row.try_get("duration_ms").map_err(sqlx_error)?,
        success: row.try_get::<i64, _>("success").map_err(sqlx_error)? != 0,
        row_count: row.try_get("row_count").map_err(sqlx_error)?,
        truncated: row.try_get::<i64, _>("truncated").map_err(sqlx_error)? != 0,
        error_summary: row.try_get("error_summary").map_err(sqlx_error)?,
    })
}

fn non_empty(value: Option<String>) -> Option<String> {
    value.and_then(|v| {
        let trimmed = v.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
}

pub fn safe_connection_name(name: String) -> Option<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return None;
    }
    let lower = trimmed.to_ascii_lowercase();
    let unsafe_markers = [
        "://",
        "password=",
        "password:",
        "pwd=",
        "secret=",
        "token=",
        "apikey=",
        "api_key=",
    ];
    if unsafe_markers.iter().any(|marker| lower.contains(marker)) {
        return None;
    }
    Some(trimmed.chars().take(160).collect())
}

pub fn redact_sql_for_storage(sql: &str) -> String {
    sql_for_storage(sql)
        .unwrap_or("<redacted-sensitive-sql>")
        .to_owned()
}

/// Preserve non-secret SQL byte-for-byte; omit credential-bearing statements.
pub fn sql_for_storage(sql: &str) -> Option<&str> {
    (!cellar_sql::sql_contains_credentials(sql)).then_some(sql)
}

fn redact_error_summary(summary: String) -> String {
    let redacted = redact_uri_like(&summary);
    let redacted = ["password", "pwd", "token", "secret", "api_key", "apikey"]
        .into_iter()
        .fold(redacted, |text, key| redact_assignment(&text, key));
    redacted.chars().take(MAX_ERROR_SUMMARY).collect()
}

fn redact_uri_like(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut token = String::new();
    for c in input.chars() {
        if c.is_whitespace() {
            flush_uri_token(&mut out, &mut token);
            out.push(c);
        } else {
            token.push(c);
        }
    }
    flush_uri_token(&mut out, &mut token);
    out
}

fn flush_uri_token(out: &mut String, token: &mut String) {
    if token.is_empty() {
        return;
    }
    if token.contains("://") {
        out.push_str("<redacted-uri>");
    } else {
        out.push_str(token);
    }
    token.clear();
}

fn redact_assignment(input: &str, key: &str) -> String {
    let needle = format!("{key}=");
    let replacement = format!("{needle}<redacted>");
    let mut output = input.to_string();
    let mut search_from = 0;
    loop {
        let lower = output[search_from..].to_ascii_lowercase();
        let Some(offset) = lower.find(&needle) else {
            return output;
        };
        let start = search_from + offset;
        let value_start = start + needle.len();
        let value_end = output[value_start..]
            .find(|c: char| c == ';' || c == ',' || c.is_whitespace())
            .map(|offset| value_start + offset)
            .unwrap_or(output.len());
        output.replace_range(start..value_end, &replacement);
        search_from = start + replacement.len();
    }
}

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or(0)
}

fn sqlx_error(err: sqlx::Error) -> CellarError {
    CellarError::Internal(err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn stores_and_filters_history_records() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("history.db");
        let store = HistoryStore::open_at(path.clone())
            .await
            .expect("history store");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }

        store
            .insert(NewQueryHistoryRecord {
                connection_id: "local-pg".into(),
                connection_name: Some("Local Postgres".into()),
                tab_id: Some("tab-1".into()),
                database: Some("app".into()),
                sql: "SELECT * FROM public.orders".into(),
                duration_ms: 12,
                success: true,
                row_count: Some(10),
                truncated: false,
                error_summary: None,
            })
            .await
            .expect("insert");
        store
            .insert(NewQueryHistoryRecord {
                connection_id: "local-pg".into(),
                connection_name: Some("postgres://user:password@host/db".into()),
                tab_id: Some("tab-2".into()),
                database: Some("app".into()),
                sql: "SELECT * FROM private_tokens".into(),
                duration_ms: 7,
                success: false,
                row_count: None,
                truncated: false,
                error_summary: Some("password=secret went wrong".into()),
            })
            .await
            .expect("insert failure");

        let records = store
            .list(QueryHistoryFilter {
                connection_id: Some("local-pg".into()),
                database: Some("app".into()),
                tab_id: None,
                search: Some("orders".into()),
                limit: Some(20),
            })
            .await
            .expect("list");

        assert_eq!(records.len(), 1);
        assert_eq!(
            records[0].connection_name.as_deref(),
            Some("Local Postgres")
        );
        assert_eq!(records[0].row_count, Some(10));
        assert!(records[0].success);

        let failures = store
            .list(QueryHistoryFilter {
                search: Some("wrong".into()),
                ..QueryHistoryFilter::default()
            })
            .await
            .expect("search errors");
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].connection_name, None);
        assert_eq!(
            failures[0].error_summary.as_deref(),
            Some("password=<redacted> went wrong")
        );
    }

    #[test]
    fn redacts_obvious_secrets() {
        assert_eq!(
            safe_connection_name("postgres://user:pass@example/db".into()),
            None
        );
        assert_eq!(
            redact_sql_for_storage("CREATE USER a PASSWORD 'secret'"),
            "<redacted-sensitive-sql>"
        );
        assert_eq!(
            redact_sql_for_storage("CREATE ROLE a PASSWORD $$secret$$"),
            "<redacted-sensitive-sql>"
        );
        assert_eq!(
            redact_sql_for_storage("SELECT password FROM users WHERE name = 'alice'"),
            "SELECT password FROM users WHERE name = 'alice'"
        );
        assert_eq!(
            redact_error_summary("failed for token=abc123; retry".into()),
            "failed for token=<redacted>; retry"
        );
        assert_eq!(
            redact_error_summary("token=abc; retry token=def api_key=ghi secret=jkl".into()),
            "token=<redacted>; retry token=<redacted> api_key=<redacted> secret=<redacted>"
        );
    }
}
