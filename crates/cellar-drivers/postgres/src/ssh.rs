//! PostgreSQL-specific TLS negotiation over the shared SSH transport.
use cellar_core::driver::{ConnectionConfig, ConnectionSecrets};
use cellar_core::error::{CellarError, CellarResult};
use tokio::time::{timeout, Duration};

pub(crate) use cellar_ssh::SshTunnel;

pub(crate) async fn open(
    config: &ConnectionConfig,
    secrets: &ConnectionSecrets,
) -> CellarResult<SshTunnel> {
    let config = config.clone();
    let tls_config = config.clone();
    SshTunnel::open_with(&config, secrets, move |mut local, remote| {
        let config = tls_config.clone();
        async move {
            // Bound only negotiation, never the lifetime of an established query.
            let mut remote = timeout(
                Duration::from_secs(30),
                crate::ssh_tls::database_stream(remote, &config),
            )
            .await
            .map_err(|_| {
                CellarError::Timeout("PostgreSQL TLS negotiation inside SSH timed out".into())
            })??;
            tokio::io::copy_bidirectional(&mut local, &mut remote)
                .await
                .map_err(|e| CellarError::Connection(format!("SSH forwarding failed: {e}")))?;
            Ok(())
        }
    })
    .await
}

#[cfg(test)]
#[path = "ssh_tests.rs"]
mod tests;
