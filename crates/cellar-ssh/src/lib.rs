//! Native SSH transport with explicit, user-confirmed host-key trust.
//! Unknown keys fail closed; only the separate confirmation API persists trust.
use std::future::Future;
use std::net::{Shutdown, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use cellar_core::driver::{ConnectionConfig, ConnectionSecrets, SshAuthMethod, SshHostConfig};
use cellar_core::error::{CellarError, CellarResult};
use russh::client::{self, Handle};
use russh::keys::{self, PrivateKeyWithHashAlg};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::{JoinHandle, JoinSet};
use tokio::time::{timeout, Duration};

mod known_hosts;
pub use known_hosts::trust_host_key;
use known_hosts::verify_host_key;

// Match sqlx's default pool acquire budget; bound stalled SSH peers and agents
// before pool acquisition begins. This is not an idle/query timeout.
const STARTUP_TIMEOUT: Duration = Duration::from_secs(30);
// Mirrors OpenSSH ServerAliveInterval=30/ServerAliveCountMax=3: a silent peer
// is probed every 30s and the session fails after 3 unanswered probes, so
// half-open sessions error instead of hanging. No inactivity_timeout is set
// because idle pooled connections are healthy.
const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(30);
const KEEPALIVE_MAX: usize = 3;

pub trait TunnelStream: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send {}
impl<T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send> TunnelStream for T {}
pub type BoxedStream = Box<dyn TunnelStream>;

#[derive(Debug, thiserror::Error)]
enum SshError {
    #[error(transparent)]
    Transport(#[from] russh::Error),
    #[error(transparent)]
    Cellar(#[from] CellarError),
}

impl From<SshError> for CellarError {
    fn from(error: SshError) -> Self {
        match error {
            SshError::Transport(error) => transport_error(error),
            SshError::Cellar(error) => error,
        }
    }
}

struct HostVerifier {
    host: String,
    port: u16,
    known_hosts: PathBuf,
}

impl client::Handler for HostVerifier {
    type Error = SshError;

    async fn check_server_key(
        &mut self,
        key: &keys::PublicKeyOrCertificate,
    ) -> Result<bool, SshError> {
        let keys::PublicKeyOrCertificate::PublicKey { key, .. } = key else {
            return Err(CellarError::Authentication(
                "SSH host certificates are not supported; provision an explicit host key".into(),
            )
            .into());
        };
        verify_host_key(&self.host, self.port, key, &self.known_hosts)?;
        Ok(true)
    }
}

// russh owns background session tasks. Shutting down the underlying socket
// also tears down nested jump channels, including during cancelled startup.
struct ShutdownSocket(std::net::TcpStream);
impl Drop for ShutdownSocket {
    fn drop(&mut self) {
        let _ = self.0.shutdown(Shutdown::Both);
    }
}

pub struct SshTunnel {
    address: SocketAddr,
    task: Mutex<Option<JoinHandle<()>>>,
    socket: ShutdownSocket,
    error: Arc<Mutex<Option<CellarError>>>,
}

impl Drop for SshTunnel {
    fn drop(&mut self) {
        self.stop();
    }
}

impl SshTunnel {
    pub fn address(&self) -> SocketAddr {
        self.address
    }

    pub fn take_error(&self) -> Option<CellarError> {
        self.error.lock().unwrap().take()
    }

    fn stop(&self) -> Option<JoinHandle<()>> {
        let task = self.task.lock().unwrap().take();
        if let Some(task) = &task {
            task.abort();
        }
        let _ = self.socket.0.shutdown(Shutdown::Both);
        task
    }

    pub async fn close(&self) {
        if let Some(task) = self.stop() {
            // Explicit close waits for the listener task; dropping its JoinSet
            // cancels per-connection forwards without awaiting them.
            let _ = task.await;
        }
    }

    pub async fn open(
        config: &ConnectionConfig,
        secrets: &ConnectionSecrets,
    ) -> CellarResult<Self> {
        Self::open_with(config, secrets, forward_bytes).await
    }

    /// The callback owns protocol negotiation and its deadlines. Established
    /// forwarding is never subject to the SSH startup deadline.
    pub async fn open_with<F, Fut>(
        config: &ConnectionConfig,
        secrets: &ConnectionSecrets,
        forward: F,
    ) -> CellarResult<Self>
    where
        F: Fn(TcpStream, BoxedStream) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = CellarResult<()>> + Send + 'static,
    {
        let known_hosts = dirs::home_dir()
            .ok_or_else(|| CellarError::invalid_config("cannot locate SSH known_hosts"))?
            .join(".ssh/known_hosts");
        timeout(
            STARTUP_TIMEOUT,
            Self::open_inner(config, secrets, &known_hosts, forward),
        )
        .await
        .map_err(|_| CellarError::Timeout("SSH connection/authentication timed out".into()))?
        .map_err(Into::into)
    }

    async fn open_inner<F, Fut>(
        config: &ConnectionConfig,
        secrets: &ConnectionSecrets,
        known_hosts: &Path,
        forward: F,
    ) -> Result<Self, SshError>
    where
        F: Fn(TcpStream, BoxedStream) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = CellarResult<()>> + Send + 'static,
    {
        let ssh = config
            .ssh
            .as_ref()
            .ok_or_else(|| CellarError::invalid_config("missing SSH config"))?;
        validate_host(&ssh.server)?;
        if let Some(jump) = &ssh.jump_host {
            validate_host(jump)?;
        }
        if config.host.is_empty() || config.port == 0 {
            return Err(
                CellarError::invalid_config("database host and nonzero port are required").into(),
            );
        }
        let first = ssh.jump_host.as_ref().unwrap_or(&ssh.server);
        let socket = TcpStream::connect((first.host.as_str(), first.port))
            .await
            .map_err(transport_error)?;
        let socket = socket.into_std().map_err(transport_error)?;
        let guard = ShutdownSocket(socket.try_clone().map_err(transport_error)?);
        let socket = TcpStream::from_std(socket).map_err(transport_error)?;
        let make_verifier = |host: &SshHostConfig| HostVerifier {
            host: host.host.clone(),
            port: host.port,
            known_hosts: known_hosts.to_owned(),
        };
        let options = Arc::new(client::Config {
            keepalive_interval: Some(KEEPALIVE_INTERVAL),
            keepalive_max: KEEPALIVE_MAX,
            ..Default::default()
        });
        let mut initial =
            client::connect_stream(options.clone(), socket, make_verifier(first)).await?;
        let (server, jump) = if let Some(jump_config) = &ssh.jump_host {
            authenticate(
                &mut initial,
                jump_config,
                secrets.jump_password.as_deref(),
                secrets.jump_key_passphrase.as_deref(),
            )
            .await?;
            let channel = initial
                .channel_open_direct_tcpip(&ssh.server.host, ssh.server.port.into(), "127.0.0.1", 0)
                .await?;
            let mut server =
                client::connect_stream(options, channel.into_stream(), make_verifier(&ssh.server))
                    .await?;
            authenticate(
                &mut server,
                &ssh.server,
                secrets.ssh_password.as_deref(),
                secrets.ssh_key_passphrase.as_deref(),
            )
            .await?;
            (server, Some(initial))
        } else {
            authenticate(
                &mut initial,
                &ssh.server,
                secrets.ssh_password.as_deref(),
                secrets.ssh_key_passphrase.as_deref(),
            )
            .await?;
            (initial, None)
        };
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .await
            .map_err(transport_error)?;
        let address = listener.local_addr().map_err(transport_error)?;
        let (target_host, target_port) = (config.host.clone(), config.port);
        let error = Arc::new(Mutex::new(None));
        let task_error = error.clone();
        let forward = Arc::new(forward);
        let server = Arc::new(server);
        let task = tokio::spawn(async move {
            // Both authenticated sessions outlive every forwarded connection.
            let _jump = jump;
            let mut forwards = JoinSet::new();
            loop {
                tokio::select! {
                    accepted = listener.accept() => {
                        let Ok((local, _)) = accepted else { break };
                        let (server, forward, error) = (server.clone(), forward.clone(), task_error.clone());
                        let host = target_host.clone();
                        // Channel opening runs per connection so a slow open
                        // never delays accepting other local connections.
                        forwards.spawn(async move {
                            let opened = timeout(
                                STARTUP_TIMEOUT,
                                server.channel_open_direct_tcpip(host, target_port.into(), "127.0.0.1", 0),
                            )
                            .await;
                            let result = match opened {
                                Ok(Ok(channel)) => forward(local, Box::new(channel.into_stream())).await,
                                Ok(Err(e)) => Err(channel_open_error(e)),
                                Err(_) => Err(CellarError::Timeout(
                                    "SSH forwarding channel did not open within 30 seconds".into(),
                                )),
                            };
                            if let Err(e) = result { *error.lock().unwrap() = Some(e); }
                        });
                    }
                    _ = forwards.join_next(), if !forwards.is_empty() => {}
                }
            }
            // JoinSet aborts outstanding forwards on listener failure or drop.
        });
        Ok(Self {
            address,
            task: Mutex::new(Some(task)),
            socket: guard,
            error,
        })
    }
}

fn validate_host(host: &SshHostConfig) -> CellarResult<()> {
    if host.host.trim().is_empty() || host.user.trim().is_empty() || host.port == 0 {
        return Err(CellarError::invalid_config(
            "SSH host, user and nonzero port are required",
        ));
    }
    if host.auth_method == SshAuthMethod::PrivateKey
        && host
            .private_key_path
            .as_deref()
            .is_none_or(|p| p.trim().is_empty())
    {
        return Err(CellarError::invalid_config(
            "SSH private key path is required",
        ));
    }
    Ok(())
}

async fn authenticate(
    session: &mut Handle<HostVerifier>,
    host: &SshHostConfig,
    password: Option<&str>,
    passphrase: Option<&str>,
) -> Result<(), SshError> {
    let success = match host.auth_method {
        SshAuthMethod::Password => session
            .authenticate_password(
                &host.user,
                password.ok_or_else(|| {
                    CellarError::Authentication("SSH password is required".into())
                })?,
            )
            .await?
            .success(),
        SshAuthMethod::PrivateKey => {
            let path = expand_key_path(
                host.private_key_path.as_deref().unwrap_or_default(),
                cfg!(windows),
                dirs::home_dir,
            )?;
            // Key parser errors are deliberately not formatted: input may be sensitive.
            let passphrase = passphrase.map(str::to_owned);
            let key = tokio::task::spawn_blocking(move || {
                keys::load_secret_key(&path, passphrase.as_deref())
            })
            .await
            .map_err(|_| CellarError::Authentication("SSH key loading task failed".into()))?
            .map_err(|_| {
                CellarError::Authentication("could not read or decrypt SSH private key".into())
            })?;
            let hash = session.best_supported_rsa_hash().await?.flatten();
            session
                .authenticate_publickey(&host.user, PrivateKeyWithHashAlg::new(Arc::new(key), hash))
                .await?
                .success()
        }
        SshAuthMethod::Agent => authenticate_agent(session, &host.user).await?,
    };
    if success {
        Ok(())
    } else {
        Err(CellarError::Authentication("SSH authentication rejected".into()).into())
    }
}

/// Expands only a leading home reference; no other environment variables or
/// `~user` forms are expanded, so saved paths cannot pull in arbitrary env.
fn expand_key_path(
    path: &str,
    windows: bool,
    home: impl FnOnce() -> Option<PathBuf>,
) -> CellarResult<PathBuf> {
    let Some(relative) = home_relative(path, windows) else {
        return Ok(PathBuf::from(path));
    };
    let home = home()
        .ok_or_else(|| CellarError::invalid_config("cannot locate SSH key home directory"))?;
    Ok(if relative.is_empty() {
        home
    } else {
        home.join(relative)
    })
}

/// Returns the remainder after `~` or `~/` (Windows: also `~\` and a
/// case-insensitive `%USERPROFILE%` prefix), with leading separators removed
/// so the join never becomes an absolute path.
fn home_relative(path: &str, windows: bool) -> Option<&str> {
    const PROFILE: &str = "%USERPROFILE%";
    let separators: &[char] = if windows { &['/', '\\'] } else { &['/'] };
    let rest = path.strip_prefix('~').or_else(|| {
        let head = path.get(..PROFILE.len())?;
        (windows && head.eq_ignore_ascii_case(PROFILE)).then(|| &path[PROFILE.len()..])
    })?;
    if rest.is_empty() {
        return Some(rest);
    }
    rest.starts_with(separators)
        .then(|| rest.trim_start_matches(separators))
}

async fn authenticate_agent(
    session: &mut Handle<HostVerifier>,
    user: &str,
) -> Result<bool, SshError> {
    #[cfg(unix)]
    let mut agent = keys::agent::client::AgentClient::connect_env()
        .await
        .map_err(|_| CellarError::Authentication("cannot connect to SSH_AUTH_SOCK agent".into()))?;
    #[cfg(windows)]
    let mut agent = keys::agent::client::AgentClient::connect_named_pipe(
        std::env::var("SSH_AUTH_SOCK").unwrap_or_else(|_| r"\\.\pipe\openssh-ssh-agent".into()),
    )
    .await
    .map_err(|_| CellarError::Authentication("cannot connect to OpenSSH agent".into()))?;
    authenticate_agent_with(session, user, &mut agent).await
}

async fn authenticate_agent_with<S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send>(
    session: &mut Handle<HostVerifier>,
    user: &str,
    agent: &mut keys::agent::client::AgentClient<S>,
) -> Result<bool, SshError> {
    let identities = agent
        .request_identities()
        .await
        .map_err(|_| CellarError::Authentication("cannot read SSH agent identities".into()))?;
    let hash = session.best_supported_rsa_hash().await?.flatten();
    for identity in identities {
        let keys::agent::AgentIdentity::PublicKey { key, .. } = identity else {
            continue;
        };
        if session
            .authenticate_publickey_with(user, key, hash, agent)
            .await
            .map_err(|_| CellarError::Authentication("SSH agent signing failed".into()))?
            .success()
        {
            return Ok(true);
        }
    }
    Ok(false)
}

async fn forward_bytes(mut local: TcpStream, mut remote: BoxedStream) -> CellarResult<()> {
    tokio::io::copy_bidirectional(&mut local, &mut remote)
        .await
        .map_err(transport_error)?;
    Ok(())
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

fn channel_open_error(error: russh::Error) -> CellarError {
    use russh::ChannelOpenFailure as Failure;
    let russh::Error::ChannelOpenFailure(reason) = error else {
        return transport_error(error);
    };
    let reason = match reason {
        Failure::AdministrativelyProhibited => {
            "administratively prohibited (check AllowTcpForwarding/PermitOpen)".into()
        }
        Failure::ConnectFailed => "the SSH server could not reach the database host".into(),
        Failure::UnknownChannelType => "direct TCP forwarding is not supported".into(),
        Failure::ResourceShortage => "the SSH server is out of resources".into(),
        Failure::Other { code, reason } => format!("code {code}: {reason}"),
    };
    CellarError::Connection(format!("SSH server refused database forwarding: {reason}"))
}

fn transport_error(error: impl std::fmt::Display) -> CellarError {
    CellarError::Connection(format!("SSH transport failed: {error}"))
}
