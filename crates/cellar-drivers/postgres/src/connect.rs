use std::any::Any;
use std::collections::HashMap;
use std::str::FromStr;

use async_trait::async_trait;
use cellar_core::driver::{Connection, ConnectionConfig, ConnectionSecrets, DriverInfo, SslMode};
use cellar_core::error::{CellarError, CellarResult};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions, PgSslMode};
use sqlx::{Error as SqlxError, PgPool, Row};
use tokio::sync::Mutex;

// ponytail: duplicated in mysql/connect.rs — sharing requires sqlx in cellar-core, which we deliberately avoid.
const DEFAULT_POOL_SIZE: u32 = 4;

/// Backend process running a registered query, so a concurrent cancel call
/// can signal it with `pg_cancel_backend`.
#[derive(Debug, Clone)]
pub(crate) struct ActiveQuery {
    pub pid: i32,
    pub database: String,
}

pub struct PgConnection {
    info: DriverInfo,
    /// Pool bound to `config.database` — the database named in the connection.
    pool: PgPool,
    /// Kept so we can open pools to *other* databases on the same server
    /// (Postgres binds one connection to one database). The password lives
    /// only in process memory for the session — never on disk.
    config: ConnectionConfig,
    password: Option<String>,
    tunnel: Option<crate::ssh::SshTunnel>,
    /// Lazily-opened pools for databases other than the default, keyed by
    /// database name. Reused across introspection and queries, closed when the
    /// connection is dropped.
    siblings: Mutex<HashMap<String, PgPool>>,
    /// In-flight statements keyed by [`Query::query_id`], registered for the
    /// duration of execution. A std mutex is fine: every critical section is
    /// a map insert/remove/clone with no await inside.
    active_queries: std::sync::Mutex<HashMap<String, ActiveQuery>>,
}

impl PgConnection {
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub fn config(&self) -> &ConnectionConfig {
        &self.config
    }

    /// Get (or open and cache) a pool bound to `database` on the same server.
    /// Returns the default pool when `database` is the connection's own
    /// database. `PgPool` is internally reference-counted, so the clone is
    /// cheap and shares the underlying connections.
    pub async fn pool_for_database(&self, database: &str) -> CellarResult<PgPool> {
        if database == self.config.database {
            return Ok(self.pool.clone());
        }
        let mut guard = self.siblings.lock().await;
        if let Some(existing) = guard.get(database) {
            return Ok(existing.clone());
        }
        let mut sibling = self.config.clone();
        sibling.database = database.to_string();
        let pool = build_pool(
            &sibling,
            self.password.as_deref(),
            DEFAULT_POOL_SIZE,
            self.tunnel.as_ref(),
        )
        .await?;
        guard.insert(database.to_string(), pool.clone());
        Ok(pool)
    }

    pub(crate) fn register_query(&self, query_id: &str, pid: i32, database: &str) {
        self.active_queries.lock().unwrap().insert(
            query_id.to_string(),
            ActiveQuery {
                pid,
                database: database.to_string(),
            },
        );
    }

    pub(crate) fn unregister_query(&self, query_id: &str) {
        self.active_queries.lock().unwrap().remove(query_id);
    }

    pub(crate) fn lookup_query(&self, query_id: &str) -> Option<ActiveQuery> {
        self.active_queries.lock().unwrap().get(query_id).cloned()
    }
}

#[async_trait]
impl Connection for PgConnection {
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
        let guard = self.siblings.lock().await;
        for pool in guard.values() {
            pool.close().await;
        }
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
) -> CellarResult<PgConnection> {
    let secrets = ConnectionSecrets {
        password: password.map(str::to_owned),
        ..Default::default()
    };
    open_pool_with_secrets(config, &secrets).await
}

pub async fn open_pool_with_secrets(
    config: &ConnectionConfig,
    secrets: &ConnectionSecrets,
) -> CellarResult<PgConnection> {
    let tunnel = if config.ssh.is_some() {
        Some(crate::ssh::open(config, secrets).await?)
    } else {
        None
    };
    let password = secrets.password.as_deref();
    let pool = build_pool(config, password, DEFAULT_POOL_SIZE, tunnel.as_ref()).await?;

    let version = sqlx::query("SELECT version() AS v")
        .fetch_one(&pool)
        .await
        .map_err(|e| {
            map_sqlx_err_for_runtime(e, "initial server version check", CellarError::connection)
        })?;
    let version: String = version
        .try_get::<String, _>("v")
        .map_err(|e| CellarError::Decode(e.to_string()))?;

    Ok(PgConnection {
        info: DriverInfo {
            // Report the config's engine, not a literal: Supabase and Neon
            // connections run through this same driver.
            engine: config.engine,
            version,
        },
        pool,
        config: config.clone(),
        password: password.map(|p| p.to_string()),
        tunnel,
        siblings: Mutex::new(HashMap::new()),
        active_queries: std::sync::Mutex::new(HashMap::new()),
    })
}

async fn build_pool(
    config: &ConnectionConfig,
    password: Option<&str>,
    max_connections: u32,
    tunnel: Option<&crate::ssh::SshTunnel>,
) -> CellarResult<PgPool> {
    let opts = connect_options(config, password, tunnel)?;
    PgPoolOptions::new()
        .max_connections(max_connections)
        .connect_with(opts)
        .await
        .map_err(|error| {
            tunnel
                .and_then(|t| t.take_error())
                .unwrap_or_else(|| map_sqlx_err_for_connect(error))
        })
}

pub(crate) fn connect_options(
    config: &ConnectionConfig,
    password: Option<&str>,
    tunnel: Option<&crate::ssh::SshTunnel>,
) -> CellarResult<PgConnectOptions> {
    // Build an escaped URL so sqlx retains its original-host pgpass lookup;
    // only then point the socket at the loopback TLS bridge.
    let mut url = url::Url::parse("postgres://localhost").expect("static PostgreSQL URL");
    match config.host.parse::<std::net::IpAddr>() {
        Ok(ip) => url
            .set_ip_host(ip)
            .map_err(|_| CellarError::invalid_config("invalid database host"))?,
        Err(_) => url
            .set_host(Some(&config.host))
            .map_err(|_| CellarError::invalid_config("invalid database host"))?,
    }
    url.set_port(Some(config.port))
        .map_err(|_| CellarError::invalid_config("invalid database port"))?;
    url.set_username(&config.user.replace('%', "%25"))
        .map_err(|_| CellarError::invalid_config("invalid database user"))?;
    url.set_path(&config.database.replace('%', "%25"));
    let mut opts = PgConnectOptions::from_str(url.as_str())
        .map_err(|_| CellarError::invalid_config("invalid PostgreSQL connection options"))?
        .host(&config.host)
        .ssl_mode(map_ssl_mode(config.ssl_mode));
    if let Some(tunnel) = tunnel {
        opts = opts
            .host("127.0.0.1")
            .port(tunnel.address().port())
            .ssl_mode(PgSslMode::Disable);
    }
    if let Some(p) = password {
        opts = opts.password(p);
    }
    if let Some(name) = config.application_name.as_deref() {
        opts = opts.application_name(name);
    }

    Ok(opts)
}

fn map_ssl_mode(mode: SslMode) -> PgSslMode {
    match mode {
        SslMode::Disable => PgSslMode::Disable,
        SslMode::Prefer => PgSslMode::Prefer,
        SslMode::Require => PgSslMode::Require,
        SslMode::VerifyCa => PgSslMode::VerifyCa,
        SslMode::VerifyFull => PgSslMode::VerifyFull,
    }
}

fn map_sqlx_err_for_connect(err: SqlxError) -> CellarError {
    let msg = err.to_string();
    match err {
        SqlxError::Database(ref db) => {
            // 28P01 invalid_password, 28000 invalid_authorization_specification
            if matches!(db.code().as_deref(), Some("28P01") | Some("28000")) {
                CellarError::Authentication(msg)
            } else {
                CellarError::Connection(msg)
            }
        }
        SqlxError::Tls(_) => CellarError::Tls(format!("TLS handshake failed: {msg}")),
        SqlxError::PoolTimedOut => CellarError::Timeout(
            "timed out while opening the Postgres connection pool; check host reachability and retry"
                .into(),
        ),
        SqlxError::PoolClosed => CellarError::NotConnected(
            "Postgres connection pool closed before the connection finished opening".into(),
        ),
        SqlxError::Io(_) => {
            CellarError::Connection(format!("could not reach Postgres: {msg}"))
        }
        _ => CellarError::Connection(msg),
    }
}

// ponytail: map_sqlx_err_for_runtime is identical in mysql/connect.rs. Sharing requires sqlx in cellar-core (unwanted dep). Leave both copies.
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

pub(crate) fn as_pg<'a>(conn: &'a dyn Connection) -> CellarResult<&'a PgConnection> {
    conn.as_any().downcast_ref::<PgConnection>().ok_or_else(|| {
        CellarError::NotConnected(format!(
            "expected postgres connection, got {}",
            conn.info().engine.as_str()
        ))
    })
}
