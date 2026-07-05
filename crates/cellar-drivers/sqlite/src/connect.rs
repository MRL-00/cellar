use std::any::Any;
use std::time::Duration;

use async_trait::async_trait;
use cellar_core::driver::{Connection, ConnectionConfig, DriverInfo};
use cellar_core::error::{CellarError, CellarResult};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Error as SqlxError, Row, SqlitePool};

// ponytail: duplicated in mysql/connect.rs — sharing requires sqlx in cellar-core, which we deliberately avoid.
const DEFAULT_POOL_SIZE: u32 = 4;

pub struct SqliteConnection {
    info: DriverInfo,
    pool: SqlitePool,
    config: ConnectionConfig,
}

impl SqliteConnection {
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub fn config(&self) -> &ConnectionConfig {
        &self.config
    }
}

#[async_trait]
impl Connection for SqliteConnection {
    fn info(&self) -> &DriverInfo {
        &self.info
    }

    async fn ping(&self) -> CellarResult<()> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(|e| {
                map_sqlx_err_for_runtime(e, "connection health check", CellarError::connection)
            })
    }

    async fn close(&self) -> CellarResult<()> {
        self.pool.close().await;
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub async fn open_pool(config: &ConnectionConfig) -> CellarResult<SqliteConnection> {
    let path = config.database.trim();
    if path.is_empty() {
        return Err(CellarError::invalid_config(
            "SQLite connections need a database file path in the database field",
        ));
    }
    if !std::path::Path::new(path).is_file() {
        // Opening a missing path would silently create an empty database;
        // a typo'd path should fail loudly instead.
        return Err(CellarError::invalid_config(format!(
            "SQLite database file not found: {path}"
        )));
    }

    let opts = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(false)
        // Concurrent readers in the pool can hit a writer's lock; wait a
        // moment instead of failing immediately with "database is locked".
        .busy_timeout(Duration::from_secs(5))
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(DEFAULT_POOL_SIZE)
        .connect_with(opts)
        .await
        .map_err(map_sqlx_err_for_connect)?;

    let version = sqlx::query("SELECT sqlite_version() AS v")
        .fetch_one(&pool)
        .await
        .map_err(|e| {
            map_sqlx_err_for_runtime(e, "initial version check", CellarError::connection)
        })?;
    let version: String = version
        .try_get::<String, _>("v")
        .map_err(|e| CellarError::Decode(e.to_string()))?;

    Ok(SqliteConnection {
        info: DriverInfo {
            engine: config.engine,
            version: format!("SQLite {version}"),
        },
        pool,
        config: config.clone(),
    })
}

fn map_sqlx_err_for_connect(err: SqlxError) -> CellarError {
    let msg = err.to_string();
    match err {
        SqlxError::Io(_) => CellarError::Connection(format!("could not open SQLite file: {msg}")),
        SqlxError::Database(_) => CellarError::Connection(msg),
        _ => CellarError::Connection(msg),
    }
}

pub(crate) fn map_sqlx_err_for_runtime(
    err: SqlxError,
    operation: &str,
    database_error: fn(String) -> CellarError,
) -> CellarError {
    let msg = err.to_string();
    match err {
        SqlxError::PoolClosed => CellarError::NotConnected(format!(
            "{operation} failed because the connection pool is closed. Reconnect and retry."
        )),
        SqlxError::PoolTimedOut => CellarError::Timeout(format!(
            "{operation} timed out waiting for an available database connection. Retry, or reconnect if it keeps happening."
        )),
        SqlxError::Io(_) => CellarError::connection(format!(
            "{operation} could not read the SQLite file: {msg}. Reconnect and retry."
        )),
        SqlxError::Database(_) => database_error(msg),
        other => database_error(other.to_string()),
    }
}

pub(crate) fn as_sqlite(conn: &dyn Connection) -> CellarResult<&SqliteConnection> {
    conn.as_any()
        .downcast_ref::<SqliteConnection>()
        .ok_or_else(|| {
            CellarError::NotConnected(format!(
                "expected sqlite connection, got {}",
                conn.info().engine.as_str()
            ))
        })
}
