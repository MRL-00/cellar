//! PostgreSQL TLS lives inside the SSH channel, before any startup/auth data.
//! sqlx speaks plaintext only over loopback to this bridge: its original remote
//! hostname must not be replaced with localhost for certificate validation.
use cellar_core::driver::{ConnectionConfig, SslMode};
use cellar_core::error::{CellarError, CellarResult};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub(crate) trait DatabaseStream: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> DatabaseStream for T {}

pub(crate) async fn database_stream<S: DatabaseStream + 'static>(
    stream: S,
    config: &ConnectionConfig,
) -> CellarResult<Box<dyn DatabaseStream>> {
    let root_cert = std::env::var_os("PGSSLROOTCERT")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            dirs::home_dir()
                .map(|home| home.join(".postgresql/root.crt"))
                .filter(|path| path.is_file())
        });
    database_stream_inner(stream, config, root_cert).await
}

async fn database_stream_inner<S: DatabaseStream + 'static>(
    mut stream: S,
    config: &ConnectionConfig,
    root_cert: Option<std::path::PathBuf>,
) -> CellarResult<Box<dyn DatabaseStream>> {
    if config.ssl_mode == SslMode::Disable {
        return Ok(Box::new(stream));
    }
    // PostgreSQL SSLRequest (length=8, code=80877103).
    stream
        .write_all(&[0, 0, 0, 8, 4, 210, 22, 47])
        .await
        .map_err(tls_error)?;
    let response = stream.read_u8().await.map_err(tls_error)?;
    match response {
        b'N' if config.ssl_mode == SslMode::Prefer => return Ok(Box::new(stream)),
        b'N' => {
            return Err(CellarError::Tls(
                "PostgreSQL server does not support TLS".into(),
            ))
        }
        b'S' => {}
        _ => {
            return Err(CellarError::Tls(
                "invalid PostgreSQL TLS negotiation response".into(),
            ))
        }
    }
    let verify_cert = matches!(config.ssl_mode, SslMode::VerifyCa | SslMode::VerifyFull);
    let mut tls = native_tls::TlsConnector::builder();
    tls.danger_accept_invalid_certs(!verify_cert)
        .danger_accept_invalid_hostnames(config.ssl_mode != SslMode::VerifyFull);
    // Preserve the libpq/sqlx root-certificate environment override. Otherwise
    // use the platform trust store; no private trust files are persisted here.
    if let Some(path) = root_cert {
        let pem = tokio::fs::read(path).await.map_err(tls_error)?;
        let certificates = native_tls::Certificate::stack_from_pem(&pem).map_err(tls_error)?;
        if certificates.is_empty() {
            return Err(CellarError::Tls(
                "PostgreSQL root certificate file is empty".into(),
            ));
        }
        for cert in certificates {
            tls.add_root_certificate(cert);
        }
    }
    match (std::env::var_os("PGSSLCERT"), std::env::var_os("PGSSLKEY")) {
        (Some(cert), Some(key)) => {
            let cert = tokio::fs::read(cert).await.map_err(tls_error)?;
            let key = tokio::fs::read(key).await.map_err(tls_error)?;
            // native-tls requires a PEM PKCS#8 private key; unsupported formats
            // fail closed instead of silently omitting client authentication.
            let identity = native_tls::Identity::from_pkcs8(&cert, &key).map_err(|_| CellarError::Tls("could not load PostgreSQL TLS client identity (PEM certificate and PKCS#8 key required)".into()))?;
            tls.identity(identity);
        }
        (None, None) => {}
        _ => {
            return Err(CellarError::Tls(
                "both PGSSLCERT and PGSSLKEY are required for PostgreSQL TLS client authentication"
                    .into(),
            ))
        }
    }
    let connector = tokio_native_tls::TlsConnector::from(tls.build().map_err(tls_error)?);
    let stream = connector
        .connect(&config.host, stream)
        .await
        .map_err(tls_error)?;
    Ok(Box::new(stream))
}

#[cfg(test)]
#[path = "ssh_tls_tests.rs"]
mod tests;

fn tls_error(error: impl std::fmt::Display) -> CellarError {
    CellarError::Tls(format!("PostgreSQL TLS inside SSH tunnel failed: {error}"))
}
