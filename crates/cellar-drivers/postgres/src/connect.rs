use std::any::Any;
use std::collections::HashMap;
use std::str::FromStr;

use async_trait::async_trait;
use cellar_core::driver::{Connection, ConnectionConfig, DriverInfo, Engine, SslMode};
use cellar_core::error::{CellarError, CellarResult};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions, PgSslMode};
use sqlx::{PgPool, Row};
use tokio::sync::Mutex;

const DEFAULT_POOL_SIZE: u32 = 4;

pub struct PgConnection {
    info: DriverInfo,
    /// Pool bound to `config.database` — the database named in the connection.
    pool: PgPool,
    /// Kept so we can open pools to *other* databases on the same server
    /// (Postgres binds one connection to one database). The password lives
    /// only in process memory for the session — never on disk.
    config: ConnectionConfig,
    password: Option<String>,
    /// Lazily-opened pools for databases other than the default, keyed by
    /// database name. Reused across introspection and queries, closed when the
    /// connection is dropped.
    siblings: Mutex<HashMap<String, PgPool>>,
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
        let pool = build_pool(&sibling, self.password.as_deref(), DEFAULT_POOL_SIZE).await?;
        guard.insert(database.to_string(), pool.clone());
        Ok(pool)
    }
}

#[async_trait]
impl Connection for PgConnection {
    fn info(&self) -> &DriverInfo {
        &self.info
    }

    async fn close(&self) -> CellarResult<()> {
        self.pool.close().await;
        let guard = self.siblings.lock().await;
        for pool in guard.values() {
            pool.close().await;
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
    let pool = build_pool(config, password, DEFAULT_POOL_SIZE).await?;

    let version = sqlx::query("SELECT version() AS v")
        .fetch_one(&pool)
        .await
        .map_err(|e| CellarError::Connection(e.to_string()))?;
    let version: String = version
        .try_get::<String, _>("v")
        .map_err(|e| CellarError::Decode(e.to_string()))?;

    Ok(PgConnection {
        info: DriverInfo {
            engine: Engine::Postgres,
            version,
        },
        pool,
        config: config.clone(),
        password: password.map(|p| p.to_string()),
        siblings: Mutex::new(HashMap::new()),
    })
}

async fn build_pool(
    config: &ConnectionConfig,
    password: Option<&str>,
    max_connections: u32,
) -> CellarResult<PgPool> {
    let mut opts = PgConnectOptions::from_str(&format!(
        "postgres://{user}@{host}:{port}/{db}",
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
    if let Some(name) = config.application_name.as_deref() {
        opts = opts.application_name(name);
    }

    PgPoolOptions::new()
        .max_connections(max_connections)
        .connect_with(opts)
        .await
        .map_err(map_sqlx_err_for_connect)
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

fn map_sqlx_err_for_connect(err: sqlx::Error) -> CellarError {
    let msg = err.to_string();
    match err {
        sqlx::Error::Database(ref db) => {
            // 28P01 invalid_password, 28000 invalid_authorization_specification
            if matches!(db.code().as_deref(), Some("28P01") | Some("28000")) {
                CellarError::Authentication(msg)
            } else {
                CellarError::Connection(msg)
            }
        }
        sqlx::Error::Tls(_) => CellarError::Tls(msg),
        sqlx::Error::PoolTimedOut | sqlx::Error::PoolClosed => CellarError::Timeout(msg),
        sqlx::Error::Io(_) => CellarError::Connection(msg),
        _ => CellarError::Connection(msg),
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
