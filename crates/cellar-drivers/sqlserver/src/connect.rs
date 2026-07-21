use std::any::Any;

use async_trait::async_trait;
use cellar_core::driver::{Connection, ConnectionConfig, DriverInfo, Engine, SslMode};
use cellar_core::error::{CellarError, CellarResult};
use tiberius::{AuthMethod, Client, Config, EncryptionLevel};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

pub(crate) type TdsClient = Client<Compat<TcpStream>>;

pub struct SqlServerConnection {
    info: DriverInfo,
    config: ConnectionConfig,
    /// `None` after the session is poisoned (failed restore / rollback) so the
    /// shared TDS client cannot be reused in an unknown database or tran state.
    client: Mutex<Option<TdsClient>>,
}

impl SqlServerConnection {
    pub(crate) fn config(&self) -> &ConnectionConfig {
        &self.config
    }

    pub(crate) async fn with_client<T>(
        &self,
        f: impl AsyncFnOnce(&mut TdsClient) -> CellarResult<T>,
    ) -> CellarResult<T> {
        let mut guard = self.client.lock().await;
        let client = guard.as_mut().ok_or_else(|| {
            CellarError::NotConnected(
                "SQL Server connection is closed; reconnect and retry".into(),
            )
        })?;
        let result = f(client).await;
        if matches!(&result, Err(err) if is_session_poison(err)) {
            // Drop the TDS client so later ops cannot run on a contaminated session.
            *guard = None;
        }
        result
    }
}

/// Errors that mean the shared session must not be reused. Chosen so the app
/// registry's `should_evict_connection` closes the open connection.
pub(crate) fn session_invalidated(detail: impl Into<String>) -> CellarError {
    CellarError::NotConnected(format!(
        "SQL Server session invalidated; reconnect and retry. {}",
        detail.into()
    ))
}

pub(crate) fn is_session_poison(err: &CellarError) -> bool {
    matches!(
        err,
        CellarError::NotConnected(msg) if msg.contains("session invalidated")
    )
}

#[async_trait]
impl Connection for SqlServerConnection {
    fn info(&self) -> &DriverInfo {
        &self.info
    }

    async fn ping(&self) -> CellarResult<()> {
        self.with_client(async |client| {
            client
                .simple_query("SELECT 1")
                .await
                .map_err(|e| map_tiberius_runtime_err(e, "connection health check"))?
                .into_row()
                .await
                .map_err(|e| map_tiberius_runtime_err(e, "connection health check"))?;
            Ok(())
        })
        .await
    }

    async fn close(&self) -> CellarResult<()> {
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub async fn open_client(
    config: &ConnectionConfig,
    password: Option<&str>,
    engine: Engine,
) -> CellarResult<SqlServerConnection> {
    let tds_config = build_config(config, password, engine)?;
    let tcp = TcpStream::connect(tds_config.get_addr())
        .await
        .map_err(|e| CellarError::connection(format!("could not reach SQL Server: {e}")))?;
    tcp.set_nodelay(true).map_err(|e| {
        CellarError::connection(format!("could not configure SQL Server TCP socket: {e}"))
    })?;

    let mut client = Client::connect(tds_config, tcp.compat_write())
        .await
        .map_err(map_tiberius_connect_err)?;
    let version = server_version(&mut client).await?;

    Ok(SqlServerConnection {
        info: DriverInfo { engine, version },
        config: config.clone(),
        client: Mutex::new(Some(client)),
    })
}

fn build_config(
    config: &ConnectionConfig,
    password: Option<&str>,
    engine: Engine,
) -> CellarResult<Config> {
    if config.host.trim().is_empty() {
        return Err(CellarError::invalid_config("SQL Server host is empty"));
    }
    if config.database.trim().is_empty() {
        return Err(CellarError::invalid_config("SQL Server database is empty"));
    }
    if config.user.trim().is_empty() {
        return Err(CellarError::invalid_config("SQL Server user is empty"));
    }

    let mut tds_config = Config::new();
    tds_config.host(&config.host);
    tds_config.port(config.port);
    tds_config.database(&config.database);
    tds_config.authentication(AuthMethod::sql_server(
        config.user.clone(),
        password.unwrap_or_default().to_string(),
    ));
    if let Some(name) = config.application_name.as_deref() {
        tds_config.application_name(name);
    }

    match (engine, config.ssl_mode) {
        (Engine::Azure, SslMode::Disable) => {
            return Err(CellarError::invalid_config(
                "Azure SQL requires TLS; choose require, verify-ca, or verify-full",
            ));
        }
        (Engine::Azure, _) => tds_config.encryption(EncryptionLevel::Required),
        (_, SslMode::Disable) => tds_config.encryption(EncryptionLevel::NotSupported),
        (_, _) => tds_config.encryption(EncryptionLevel::Required),
    }

    if matches!(config.ssl_mode, SslMode::Prefer | SslMode::Require) {
        tds_config.trust_cert();
    }

    Ok(tds_config)
}

async fn server_version(client: &mut TdsClient) -> CellarResult<String> {
    let row = client
        .simple_query("SELECT @@VERSION AS version")
        .await
        .map_err(map_tiberius_connect_err)?
        .into_row()
        .await
        .map_err(map_tiberius_connect_err)?
        .ok_or_else(|| CellarError::connection("SQL Server did not return a version row"))?;
    row.try_get::<&str, _>("version")
        .map_err(|e| CellarError::decode(e.to_string()))?
        .map(ToOwned::to_owned)
        .ok_or_else(|| CellarError::decode("SQL Server returned NULL for @@VERSION"))
}

fn map_tiberius_connect_err(err: tiberius::error::Error) -> CellarError {
    let msg = err.to_string();
    let lower = msg.to_lowercase();
    if lower.contains("login failed") || lower.contains("authentication") {
        CellarError::Authentication(msg)
    } else if lower.contains("tls") || lower.contains("certificate") || lower.contains("encryption")
    {
        CellarError::Tls(msg)
    } else {
        CellarError::Connection(msg)
    }
}

pub(crate) fn map_tiberius_runtime_err(
    err: tiberius::error::Error,
    operation: &str,
) -> CellarError {
    let msg = err.to_string();
    let lower = msg.to_lowercase();
    if lower.contains("timed out") || lower.contains("timeout") {
        CellarError::Timeout(format!("{operation} timed out: {msg}"))
    } else if lower.contains("closed") || lower.contains("connection reset") {
        CellarError::NotConnected(format!(
            "{operation} failed because the SQL Server connection is closed: {msg}"
        ))
    } else if lower.contains("tls") || lower.contains("certificate") || lower.contains("encryption")
    {
        CellarError::Tls(format!(
            "{operation} failed because TLS is not usable: {msg}"
        ))
    } else {
        CellarError::query(msg)
    }
}

pub(crate) fn as_sqlserver<'a>(conn: &'a dyn Connection) -> CellarResult<&'a SqlServerConnection> {
    conn.as_any()
        .downcast_ref::<SqlServerConnection>()
        .ok_or_else(|| {
            CellarError::NotConnected(format!(
                "expected sqlserver connection, got {}",
                conn.info().engine.as_str()
            ))
        })
}
