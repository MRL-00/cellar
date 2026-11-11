//! SQLx has no public custom MySQL socket or independent TLS-hostname option.
//! Terminate TLS on both sides: SQLx keeps its TLS-aware authentication and
//! packets are unchanged. Prefer's no-TLS remote fallback additionally bridges
//! authentication using MySQL's RSA protocol; local TLS is still pinned.
use cellar_core::driver::{ConnectionConfig, SslMode};
use cellar_core::error::{CellarError, CellarResult};
use cellar_ssh::BoxedStream;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::{rustls, TlsAcceptor};

const CLIENT_SSL: u16 = 0x0800;

#[path = "ssh_plain_auth.rs"]
mod plain_auth;

#[derive(Clone)]
pub(crate) struct TlsBridge {
    host: String,
    mode: SslMode,
    remote: tokio_native_tls::TlsConnector,
    local: TlsAcceptor,
    certificate: Vec<u8>,
}

impl TlsBridge {
    pub(crate) fn new(config: &ConnectionConfig) -> CellarResult<Self> {
        let mut remote = native_tls::TlsConnector::builder();
        configure_remote(&mut remote, config.ssl_mode);
        Self::with_connector(config, remote.build().map_err(tls_error)?)
    }

    fn with_connector(
        config: &ConnectionConfig,
        remote: native_tls::TlsConnector,
    ) -> CellarResult<Self> {
        let identity =
            rcgen::generate_simple_self_signed(vec!["127.0.0.1".into()]).map_err(tls_error)?;
        let certificate = identity.cert.pem().into_bytes();
        let key = rustls::pki_types::PrivatePkcs8KeyDer::from(identity.key_pair.serialize_der());
        let local = rustls::ServerConfig::builder_with_provider(std::sync::Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .map_err(tls_error)?
        .with_no_client_auth()
        .with_single_cert(vec![identity.cert.der().clone()], key.into())
        .map_err(tls_error)?;
        Ok(Self {
            host: config.host.clone(),
            mode: config.ssl_mode,
            remote: remote.into(),
            local: TlsAcceptor::from(std::sync::Arc::new(local)),
            certificate,
        })
    }

    pub(crate) fn local_options(
        &self,
        options: sqlx::mysql::MySqlConnectOptions,
    ) -> sqlx::mysql::MySqlConnectOptions {
        options.ssl_ca_from_pem(self.certificate.clone()).ssl_mode(
            if self.mode == SslMode::Disable {
                sqlx::mysql::MySqlSslMode::Disabled
            } else {
                sqlx::mysql::MySqlSslMode::VerifyIdentity
            },
        )
    }

    pub(crate) async fn forward(&self, local: TcpStream, remote: BoxedStream) -> CellarResult<()> {
        // Match shared SSH/PG startup's 30-second budget. Only connection
        // startup is bounded: queries and idle established sessions are not.
        let ready = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            self.prepare(local, remote),
        )
        .await
        .map_err(|_| {
            CellarError::Timeout(
                "MySQL SSH handshake and authentication timed out after 30 seconds".into(),
            )
        })??;
        if let Some((mut local, mut remote)) = ready {
            tokio::io::copy_bidirectional(&mut local, &mut remote)
                .await
                .map_err(io_error)?;
        }
        Ok(())
    }

    async fn prepare(
        &self,
        mut local: TcpStream,
        mut remote: BoxedStream,
    ) -> CellarResult<Option<(BoxedStream, BoxedStream)>> {
        // Only pre-authentication packets are buffered. The classic protocol
        // header's 24-bit length bounds each allocation to 2^24-1 bytes (+4).
        // https://dev.mysql.com/doc/dev/mysql-server/latest/page_protocol_basic_packets.html
        let mut greeting = read_packet(&mut remote).await?;
        if greeting.get(4) == Some(&0xff) {
            local.write_all(&greeting).await.map_err(io_error)?;
            return Ok(None);
        }
        let flags = greeting_flags(&greeting)?;
        let supports_tls =
            u16::from_le_bytes([greeting[flags], greeting[flags + 1]]) & CLIENT_SSL != 0;
        if !supports_tls && !matches!(self.mode, SslMode::Disable | SslMode::Prefer) {
            return Err(CellarError::Tls(
                "MySQL server does not support required TLS".into(),
            ));
        }
        // Validate the complete modern greeting before handing it to SQLx's
        // decoder; truncated challenge/plugin fields must not reach its parser.
        let handshake = plain_auth::Handshake::parse(&greeting, flags)?;
        if self.mode != SslMode::Disable {
            handshake.check(supports_tls)?;
        }
        if self.mode == SslMode::Disable {
            greeting[flags + 1] &= !0x08;
        } else {
            // Even when the remote lacks TLS, SQLx authenticates the local
            // ephemeral identity. Never use SQLx Preferred (it ignores pins).
            greeting[flags + 1] |= 0x08;
        }
        local.write_all(&greeting).await.map_err(io_error)?;
        let request = read_packet(&mut local).await?;
        if request[3] != 1 || request.len() < 8 {
            return Err(protocol_error());
        }
        let wants_tls = u16::from_le_bytes([request[4], request[5]]) & CLIENT_SSL != 0;
        if wants_tls {
            // Protocol 4.1 SSLRequest contains precisely 32 bytes and no login.
            if request.len() != 36 || self.mode == SslMode::Disable {
                return Err(protocol_error());
            }
            let mut remote: BoxedStream = if supports_tls {
                remote.write_all(&request).await.map_err(io_error)?;
                remote.flush().await.map_err(io_error)?;
                // Verify the original identity before accepting any login.
                // TLS failure is never retried in plaintext, even in Prefer.
                Box::new(
                    self.remote
                        .connect(&self.host, remote)
                        .await
                        .map_err(tls_error)?,
                )
            } else {
                remote
            };
            let mut local = self.local.accept(local).await.map_err(tls_error)?;
            plain_auth::authenticate(&mut local, &mut remote, handshake, supports_tls).await?;
            Ok(Some((Box::new(local), remote)))
        } else {
            if self.mode != SslMode::Disable {
                return Err(CellarError::Tls(
                    "MySQL client did not negotiate TLS".into(),
                ));
            }
            remote.write_all(&request).await.map_err(io_error)?;
            Ok(Some((Box::new(local), remote)))
        }
    }
}

/// Standard client SSL-mode semantics, matching libpq/MySQL clients and the
/// direct SQLx path: Prefer and Require encrypt without authenticating the
/// server certificate (Prefer also accepts a server without TLS); VerifyCa
/// checks the chain; VerifyFull also checks the original database host name.
fn configure_remote(builder: &mut native_tls::TlsConnectorBuilder, mode: SslMode) {
    builder.danger_accept_invalid_certs(!matches!(mode, SslMode::VerifyCa | SslMode::VerifyFull));
    builder.danger_accept_invalid_hostnames(mode != SslMode::VerifyFull);
}

fn greeting_flags(packet: &[u8]) -> CellarResult<usize> {
    if packet.get(4) != Some(&10) || packet[3] != 0 {
        return Err(protocol_error());
    }
    let version_end = packet[5..]
        .iter()
        .position(|b| *b == 0)
        .ok_or_else(protocol_error)?
        + 5;
    // version NUL, connection id (4), scramble (8), filler (1).
    let flags = version_end + 14;
    if packet.len() < flags + 18 {
        return Err(protocol_error());
    }
    Ok(flags)
}

async fn read_packet<S: AsyncRead + Unpin + ?Sized>(stream: &mut S) -> CellarResult<Vec<u8>> {
    let mut header = [0; 4];
    stream.read_exact(&mut header).await.map_err(io_error)?;
    let length = u32::from_le_bytes([header[0], header[1], header[2], 0]) as usize;
    let mut packet = vec![0; length + 4];
    packet[..4].copy_from_slice(&header);
    stream
        .read_exact(&mut packet[4..])
        .await
        .map_err(io_error)?;
    Ok(packet)
}

fn protocol_error() -> CellarError {
    CellarError::Connection("invalid MySQL handshake packet through SSH tunnel".into())
}
fn io_error(error: std::io::Error) -> CellarError {
    CellarError::Connection(format!("MySQL SSH forwarding failed: {error}"))
}
fn tls_error(error: impl std::fmt::Display) -> CellarError {
    CellarError::Tls(format!("MySQL SSH TLS handshake failed: {error}"))
}

#[cfg(test)]
#[path = "ssh_tls_tests.rs"]
mod tests;
