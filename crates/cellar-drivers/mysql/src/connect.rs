use std::any::Any;

use crate::ssh_tls::TlsBridge;
use async_trait::async_trait;
use cellar_core::driver::{Connection, ConnectionConfig, ConnectionSecrets, DriverInfo, SslMode};
use cellar_core::error::{CellarError, CellarResult};
use cellar_ssh::SshTunnel;
use sqlx::mysql::{MySqlConnectOptions, MySqlPoolOptions, MySqlSslMode};
use sqlx::{Error as SqlxError, MySqlPool, Row};

const DEFAULT_POOL_SIZE: u32 = 4;

pub struct MySqlConnection {
    info: DriverInfo,
    pool: MySqlPool,
    config: ConnectionConfig,
    tunnel: Option<SshTunnel>,
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
        if let Some(tunnel) = &self.tunnel {
            tunnel.close().await;
        }
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
    open_pool_with_secrets(
        config,
        &ConnectionSecrets {
            password: password.map(str::to_owned),
            ..Default::default()
        },
    )
    .await
}

pub async fn open_pool_with_secrets(
    config: &ConnectionConfig,
    secrets: &ConnectionSecrets,
) -> CellarResult<MySqlConnection> {
    let mut opts = connect_options(config, secrets.password.as_deref());
    let tunnel = if config.ssh.is_some() {
        let tunnel = if config.ssl_mode == SslMode::Disable {
            SshTunnel::open(config, secrets).await?
        } else {
            let bridge = TlsBridge::new(config)?;
            opts = bridge.local_options(opts);
            SshTunnel::open_with(config, secrets, move |local, remote| {
                let bridge = bridge.clone();
                async move { bridge.forward(local, remote).await }
            })
            .await?
        };
        opts = opts
            .host(&tunnel.address().ip().to_string())
            .port(tunnel.address().port());
        Some(tunnel)
    } else {
        None
    };
    let pool = MySqlPoolOptions::new()
        .max_connections(DEFAULT_POOL_SIZE)
        .connect_with(opts)
        .await
        .map_err(|error| {
            tunnel
                .as_ref()
                .and_then(SshTunnel::take_error)
                .unwrap_or_else(|| map_sqlx_err_for_connect(error))
        })?;

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
            // Report the config's engine, not a literal: PlanetScale
            // connections run through this same driver.
            engine: config.engine,
            version,
        },
        pool,
        config: config.clone(),
        tunnel,
    })
}

fn connect_options(config: &ConnectionConfig, password: Option<&str>) -> MySqlConnectOptions {
    let mut opts = MySqlConnectOptions::new()
        .host(&config.host)
        .port(config.port)
        .username(&config.user)
        .ssl_mode(map_ssl_mode(config.ssl_mode));
    // An empty database means none selected, as with the former URL path.
    if !config.database.is_empty() {
        opts = opts.database(&config.database);
    }
    if let Some(p) = password {
        opts = opts.password(p);
    }

    opts
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

pub(crate) fn as_mysql(conn: &dyn Connection) -> CellarResult<&MySqlConnection> {
    conn.as_any()
        .downcast_ref::<MySqlConnection>()
        .ok_or_else(|| {
            CellarError::NotConnected(format!(
                "expected mysql connection, got {}",
                conn.info().engine.as_str()
            ))
        })
}
