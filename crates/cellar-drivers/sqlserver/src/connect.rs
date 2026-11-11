use std::any::Any;
use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use cellar_core::driver::{
    Connection, ConnectionConfig, ConnectionSecrets, DriverInfo, Engine, SslMode,
};
use cellar_core::error::{CellarError, CellarResult};
use cellar_ssh::SshTunnel;
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
    /// Set by `close()` without waiting for the client lock; an operation that
    /// holds the lock drops the client when it finishes and new ones refuse.
    closed: AtomicBool,
    tunnel: Option<SshTunnel>,
}

impl SqlServerConnection {
    pub(crate) fn config(&self) -> &ConnectionConfig {
        &self.config
    }

    pub(crate) async fn with_client<T>(
        &self,
        f: impl AsyncFnOnce(&mut TdsClient) -> CellarResult<T>,
    ) -> CellarResult<T> {
        let closed = || {
            CellarError::NotConnected("SQL Server connection is closed; reconnect and retry".into())
        };
        if self.closed.load(Ordering::Acquire) {
            return Err(closed());
        }
        let mut guard = self.client.lock().await;
        if self.closed.load(Ordering::Acquire) {
            *guard = None;
            return Err(closed());
        }
        let client = guard.as_mut().ok_or_else(closed)?;
        let result = f(client).await;
        if self.closed.load(Ordering::Acquire)
            || matches!(&result, Err(err) if is_session_poison(err))
        {
            // Drop the TDS client so later ops cannot run on a closed or
            // contaminated session.
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
        // Never wait for an in-flight operation. If one holds the client, it
        // drops it on completion (see `with_client`); aborting the tunnel
        // below also kills its socket so the operation fails promptly.
        self.closed.store(true, Ordering::Release);
        if let Ok(mut client) = self.client.try_lock() {
            // Release the database socket before tearing down its transport.
            client.take();
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

pub async fn open_client(
    config: &ConnectionConfig,
    password: Option<&str>,
    engine: Engine,
) -> CellarResult<SqlServerConnection> {
    let secrets = ConnectionSecrets {
        password: password.map(str::to_owned),
        ..Default::default()
    };
    open_client_with_secrets(config, &secrets, engine).await
}

pub async fn open_client_with_secrets(
    config: &ConnectionConfig,
    secrets: &ConnectionSecrets,
    engine: Engine,
) -> CellarResult<SqlServerConnection> {
    // Azure SQL redirects are incompatible with this fixed-target tunnel.
    // Reject saved Azure tunnel settings rather than connecting directly.
    if engine == Engine::Azure || config.engine == Engine::Azure {
        config.reject_ssh()?;
    }
    let tds_config = build_config(config, secrets.password.as_deref(), engine)?;
    let tunnel = if config.ssh.is_some() {
        Some(SshTunnel::open(config, secrets).await?)
    } else {
        None
    };
    // Only the TCP destination changes. Tiberius retains the original host for
    // certificate verification, SNI, and the Azure SQL login server name.
    let tcp = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        if let Some(tunnel) = &tunnel {
            TcpStream::connect(tunnel.address()).await
        } else {
            TcpStream::connect(tds_config.get_addr()).await
        }
    })
    .await
    .map_err(|_| CellarError::Timeout("SQL Server TCP connection timed out".into()))?
    .map_err(|e| CellarError::connection(format!("could not reach SQL Server: {e}")))?;
    tcp.set_nodelay(true).map_err(|e| {
        CellarError::connection(format!("could not configure SQL Server TCP socket: {e}"))
    })?;

    // Tiberius accepts a supplied stream and returns Routing as an error; it
    // never opens a redirect socket itself. Do not retry redirects over
    // direct TCP, which would bypass SSH.
    let mut client = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        Client::connect(tds_config, tcp.compat_write()),
    )
    .await
    .map_err(|_| CellarError::Timeout("SQL Server TLS/login negotiation timed out".into()))?
    .map_err(|error| {
        tunnel
            .as_ref()
            .and_then(SshTunnel::take_error)
            .unwrap_or_else(|| map_tiberius_connect_err(error))
    })?;
    let version = server_version(&mut client).await?;

    Ok(SqlServerConnection {
        info: DriverInfo { engine, version },
        config: config.clone(),
        client: Mutex::new(Some(client)),
        closed: AtomicBool::new(false),
        tunnel,
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
    if matches!(err, tiberius::error::Error::Routing { .. }) {
        return CellarError::Connection(
            "SQL Server requested a redirect; redirects are not followed".into(),
        );
    }
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

#[cfg(test)]
#[path = "connect_tests.rs"]
mod tests;

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
