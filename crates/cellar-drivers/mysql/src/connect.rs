use std::any::Any;
use std::str::FromStr;

use async_trait::async_trait;
use cellar_core::driver::{Connection, ConnectionConfig, DriverInfo, Engine, SslMode};
use cellar_core::error::{CellarError, CellarResult};
use sqlx::mysql::{MySqlConnectOptions, MySqlPoolOptions, MySqlSslMode};
use sqlx::{Error as SqlxError, MySqlPool, Row};

const DEFAULT_POOL_SIZE: u32 = 4;

pub struct MySqlConnection {
    info: DriverInfo,
    pool: MySqlPool,
    config: ConnectionConfig,
}

impl MySqlConnection {
    pub fn pool(&self) -> &MySqlPool {
        &self.pool
    }

    pub fn config(&self) -> &ConnectionConfig {
        &self.config
    }
}

#[async_trait]
impl Connection for MySqlConnection {
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

pub async fn open_pool(
    config: &ConnectionConfig,
    password: Option<&str>,
) -> CellarResult<MySqlConnection> {
    let pool = build_pool(config, password, DEFAULT_POOL_SIZE).await?;

    let version = sqlx::query("SELECT VERSION() AS v")
        .fetch_one(&pool)
        .await
        .map_err(|e| {
            map_sqlx_err_for_runtime(e, "initial server version check", CellarError::connection)
        })?;
    let version: String = version
        .try_get::<String, _>("v")
        .map_err(|e| CellarError::Decode(e.to_string()))?;

    Ok(MySqlConnection {
        info: DriverInfo {
            engine: Engine::MySql,
            version,
        },
        pool,
        config: config.clone(),
    })
}

async fn build_pool(
    config: &ConnectionConfig,
    password: Option<&str>,
    max_connections: u32,
) -> CellarResult<MySqlPool> {
    let mut opts = MySqlConnectOptions::from_str(&format!(
        "mysql://{user}@{host}:{port}/{db}",
        user = config.user,
        host = config.host,
        port = config.port,
        db = config.database,
    ))
    .map_err(|e| CellarError::invalid_config(e.to_string()))?
    .ssl_mode(map_ssl_mode(config.ssl_mode));
    if let Some(p) = password {
        opts = opts.password(p);
    }

    MySqlPoolOptions::new()
        .max_connections(max_connections)
        .connect_with(opts)
        .await
        .map_err(map_sqlx_err_for_connect)
}

fn map_ssl_mode(mode: SslMode) -> MySqlSslMode {
    match mode {
        SslMode::Disable => MySqlSslMode::Disabled,
        SslMode::Prefer => MySqlSslMode::Preferred,
        SslMode::Require => MySqlSslMode::Required,
        SslMode::VerifyCa => MySqlSslMode::VerifyCa,
        SslMode::VerifyFull => MySqlSslMode::VerifyIdentity,
    }
}

fn map_sqlx_err_for_connect(err: SqlxError) -> CellarError {
    let msg = err.to_string();
    match err {
        SqlxError::Database(ref db) => {
            let code_str = db.code().map(|c| c.to_string());
            let code = code_str.as_deref().unwrap_or("");
            // MySQL error codes: 1045 access denied
            if code == "1045" || msg.to_lowercase().contains("access denied") {
                CellarError::Authentication(msg)
            } else {
                CellarError::Connection(msg)
            }
        }
        SqlxError::Tls(_) => CellarError::Tls(format!("TLS handshake failed: {msg}")),
        SqlxError::PoolTimedOut => CellarError::Timeout(
            "timed out while opening the MySQL connection pool; check host reachability and retry"
                .into(),
        ),
        SqlxError::PoolClosed => CellarError::NotConnected(
            "MySQL connection pool closed before the connection finished opening".into(),
        ),
        SqlxError::Io(_) => CellarError::Connection(format!("could not reach MySQL: {msg}")),
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
            "{operation} lost the database connection: {msg}. Reconnect and retry."
        )),
        SqlxError::Tls(_) => CellarError::Tls(format!(
            "{operation} failed because the TLS session is no longer usable: {msg}. Reconnect and retry."
        )),
        SqlxError::Database(_) => database_error(msg),
        other => database_error(other.to_string()),
    }
}

pub(crate) fn as_mysql<'a>(conn: &'a dyn Connection) -> CellarResult<&'a MySqlConnection> {
    conn.as_any().downcast_ref::<MySqlConnection>().ok_or_else(|| {
        CellarError::NotConnected(format!(
            "expected mysql connection, got {}",
            conn.info().engine.as_str()
        ))
    })
}
