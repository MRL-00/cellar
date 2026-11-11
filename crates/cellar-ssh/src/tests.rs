use super::*;
use cellar_core::driver::{Engine, SshTunnelConfig, SslMode};
use russh::keys::ssh_key::{private::Ed25519Keypair, PrivateKey, PublicKey};
use russh::server::{self, Auth, ChannelOpenHandle, Session};
use russh::Channel;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn key(seed: u8) -> PrivateKey {
    PrivateKey::from(Ed25519Keypair::from_seed(&[seed; 32]))
}

fn host(port: u16, user: &str) -> SshHostConfig {
    SshHostConfig {
        host: "127.0.0.1".into(),
        port,
        user: user.into(),
        auth_method: SshAuthMethod::Password,
        private_key_path: None,
    }
}

fn config(server: SshHostConfig, target: SocketAddr) -> ConnectionConfig {
    ConnectionConfig {
        id: "ssh-test".into(),
        name: "SSH test".into(),
        engine: Engine::Postgres,
        host: target.ip().to_string(),
        port: target.port(),
        database: "postgres".into(),
        user: "postgres".into(),
        ssl_mode: SslMode::Disable,
        env_tag: None,
        application_name: None,
        color: None,
        ssh: Some(SshTunnelConfig {
            server,
            jump_host: None,
        }),
    }
}

#[derive(Clone)]
struct MockSsh {
    user: &'static str,
    password: &'static str,
    public_key: Option<PublicKey>,
}

impl server::Handler for MockSsh {
    type Error = russh::Error;

    async fn auth_password(&mut self, user: &str, password: &str) -> Result<Auth, Self::Error> {
        Ok(if self.user == user && self.password == password {
            Auth::Accept
        } else {
            Auth::reject()
        })
    }

    async fn auth_publickey(
        &mut self,
        user: &str,
        public_key: &PublicKey,
    ) -> Result<Auth, Self::Error> {
        Ok(
            if user == self.user
                && self
                    .public_key
                    .as_ref()
                    .is_some_and(|key| key.key_data() == public_key.key_data())
            {
                Auth::Accept
            } else {
                Auth::reject()
            },
        )
    }

    async fn channel_open_direct_tcpip(
        &mut self,
        channel: Channel<server::Msg>,
        host: &str,
        port: u32,
        _: &str,
        _: u32,
        reply: ChannelOpenHandle,
        _: &mut Session,
    ) -> Result<(), Self::Error> {
        // Test servers never connect to external endpoints. Returning without
        // accepting drops `reply`, and russh's ChannelOpenHandle Drop sends
        // ChannelOpenFailure::AdministrativelyProhibited to the client: this
        // is how the mock refuses a channel, not a silent hang.
        if host != "127.0.0.1" {
            return Ok(());
        }
        let mut target = TcpStream::connect((host, port as u16)).await?;
        reply.accept().await;
        tokio::spawn(async move {
            let mut stream = channel.into_stream();
            let _ = tokio::io::copy_bidirectional(&mut stream, &mut target).await;
        });
        Ok(())
    }
}

struct TestServer {
    port: u16,
    task: JoinHandle<()>,
}
impl Drop for TestServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn start_server(seed: u8, handler: MockSsh) -> TestServer {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let options = Arc::new(server::Config {
        keys: vec![key(seed)],
        auth_rejection_time: Duration::ZERO,
        auth_rejection_time_initial: Some(Duration::ZERO),
        ..Default::default()
    });
    let task = tokio::spawn(async move {
        let mut sessions = JoinSet::new();
        loop {
            tokio::select! {
                accepted = listener.accept() => {
                    let Ok((socket, _)) = accepted else { break };
                    let options = options.clone(); let handler = handler.clone();
                    sessions.spawn(async move {
                        if let Ok(session) = server::run_stream(options, socket, handler).await { let _ = session.await; }
                    });
                }
                _ = sessions.join_next(), if !sessions.is_empty() => {}
            }
        }
    });
    TestServer { port, task }
}

async fn echo() -> TestServer {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let task = tokio::spawn(async move {
        let mut streams = JoinSet::new();
        loop {
            tokio::select! {
                accepted = listener.accept() => {
                    let Ok((mut stream, _)) = accepted else { break };
                    streams.spawn(async move {
                        let (mut read, mut write) = stream.split();
                        let _ = tokio::io::copy(&mut read, &mut write).await;
                    });
                }
                _ = streams.join_next(), if !streams.is_empty() => {}
            }
        }
    });
    TestServer { port, task }
}

fn trust(path: &Path, servers: &[(&TestServer, u8)]) {
    let text: String = servers
        .iter()
        .map(|(server, seed)| {
            format!(
                "[127.0.0.1]:{} {}\n",
                server.port,
                key(*seed).public_key().to_openssh().unwrap()
            )
        })
        .collect();
    std::fs::write(path, text).unwrap();
}

async fn roundtrip(tunnel: &SshTunnel) -> TcpStream {
    assert!(tunnel.address().ip().is_loopback());
    let mut stream = TcpStream::connect(tunnel.address()).await.unwrap();
    stream.write_all(b"through SSH").await.unwrap();
    let mut data = [0; 11];
    timeout(Duration::from_secs(5), stream.read_exact(&mut data))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(&data, b"through SSH");
    stream
}

#[tokio::test]
async fn password_forward_multiple_connections_and_drop_teardown() {
    let target = echo().await;
    let server = start_server(
        1,
        MockSsh {
            user: "server-user",
            password: "server-password",
            public_key: None,
        },
    )
    .await;
    let temp = tempfile::tempdir().unwrap();
    let known = temp.path().join("known_hosts");
    trust(&known, &[(&server, 1)]);
    let config = config(
        host(server.port, "server-user"),
        ([127, 0, 0, 1], target.port).into(),
    );
    let secrets = ConnectionSecrets {
        ssh_password: Some("server-password".into()),
        ..Default::default()
    };
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let callback_calls = calls.clone();
    let tunnel = SshTunnel::open_inner(&config, &secrets, &known, move |local, remote| {
        callback_calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        forward_bytes(local, remote)
    })
    .await
    .unwrap();
    let mut first = roundtrip(&tunnel).await;
    let _second = roundtrip(&tunnel).await;
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 2);
    // Once channels are established, an idle/query lifetime is not startup.
    tokio::time::pause();
    tokio::time::advance(STARTUP_TIMEOUT * 2).await;
    tokio::time::resume();
    first.write_all(b"still alive").await.unwrap();
    let mut reply = [0; 11];
    timeout(Duration::from_secs(5), first.read_exact(&mut reply))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(&reply, b"still alive");
    let address = tunnel.address();
    drop(tunnel);
    let mut byte = [0];
    let closed = timeout(Duration::from_secs(5), first.read(&mut byte))
        .await
        .unwrap();
    assert!(matches!(closed, Ok(0) | Err(_)));
    // The abort is asynchronous, so yield to observe listener destruction.
    tokio::task::yield_now().await;
    assert!(TcpStream::connect(address).await.is_err());
}

#[tokio::test]
async fn jump_password_and_independent_encrypted_private_key_auth() {
    let target = echo().await;
    let client_key = key(4);
    let server = start_server(
        2,
        MockSsh {
            user: "key-user",
            password: "unused",
            public_key: Some(client_key.public_key().clone()),
        },
    )
    .await;
    let jump = start_server(
        3,
        MockSsh {
            user: "jump-user",
            password: "jump-password",
            public_key: None,
        },
    )
    .await;
    let temp = tempfile::tempdir().unwrap();
    let known = temp.path().join("known_hosts");
    trust(&known, &[(&server, 2), (&jump, 3)]);
    let key_path = temp.path().join("private_key");
    let encrypted = client_key
        .encrypt(&mut rand::rng(), "key-passphrase")
        .unwrap();
    std::fs::write(
        &key_path,
        encrypted
            .to_openssh(keys::ssh_key::LineEnding::LF)
            .unwrap()
            .as_bytes(),
    )
    .unwrap();
    let mut server_host = host(server.port, "key-user");
    server_host.auth_method = SshAuthMethod::PrivateKey;
    server_host.private_key_path = Some(key_path.to_str().unwrap().into());
    let mut config = config(server_host, ([127, 0, 0, 1], target.port).into());
    config.ssh.as_mut().unwrap().jump_host = Some(host(jump.port, "jump-user"));
    let mut secrets = ConnectionSecrets {
        jump_password: Some("wrong".into()),
        ssh_key_passphrase: Some("key-passphrase".into()),
        ..Default::default()
    };
    assert!(
        SshTunnel::open_inner(&config, &secrets, &known, forward_bytes)
            .await
            .is_err()
    );
    secrets.jump_password = Some("jump-password".into());
    let tunnel = timeout(
        Duration::from_secs(15),
        SshTunnel::open_inner(&config, &secrets, &known, forward_bytes),
    )
    .await
    .unwrap()
    .unwrap();
    let _stream = roundtrip(&tunnel).await;
    tunnel.close().await;
    assert!(TcpStream::connect(tunnel.address()).await.is_err());
    assert!(SshTunnel::open_inner(
        &config,
        &ConnectionSecrets {
            jump_password: Some("jump-password".into()),
            ssh_key_passphrase: Some("wrong".into()),
            ..Default::default()
        },
        &known,
        forward_bytes,
    )
    .await
    .is_err());
}

#[test]
fn trust_rejects_unknown_changed_and_revoked_keys() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("known_hosts");
    let public = key(1).public_key().clone();
    let other = key(2).public_key().clone();
    std::fs::write(&path, "").unwrap();
    assert!(verify_host_key("db", 22, &public, &path).is_err());
    std::fs::write(&path, format!("db {}\n", public.to_openssh().unwrap())).unwrap();
    assert!(verify_host_key("db", 22, &public, &path).is_ok());
    assert!(verify_host_key("db", 22, &other, &path).is_err());
    std::fs::write(
        &path,
        format!(
            "db {}\n@revoked\talias\t{}\n",
            public.to_openssh().unwrap(),
            public.to_openssh().unwrap()
        ),
    )
    .unwrap();
    assert!(verify_host_key("db", 22, &public, &path)
        .unwrap_err()
        .to_string()
        .contains("revoked"));
}

#[test]
fn trust_accepts_whitespace_and_hashed_host_entries() {
    use hmac::Mac;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("known_hosts");
    let public = key(1).public_key().clone();
    let openssh = public.to_openssh().unwrap();
    for separator in ["\t", "   ", " \t "] {
        let entry = format!(
            "  [db]:2222{separator}{}\n",
            openssh.replace(' ', separator)
        );
        std::fs::write(&path, entry).unwrap();
        assert!(verify_host_key("db", 2222, &public, &path).is_ok());
        assert!(verify_host_key("db", 22, &public, &path).is_err());
    }
    let salt = b"known-hosts-test-salt".to_vec();
    let mut hash = [0; 20];
    hash.copy_from_slice(
        &hmac::Hmac::<sha1::Sha1>::new_from_slice(&salt)
            .unwrap()
            .chain_update(b"[db]:2222")
            .finalize()
            .into_bytes(),
    );
    let pattern = keys::ssh_key::known_hosts::HostPatterns::HashedName { salt, hash }.to_string();
    std::fs::write(
        &path,
        format!("{pattern}\t{}\n", openssh.replace(' ', "  ")),
    )
    .unwrap();
    assert!(verify_host_key("db", 2222, &public, &path).is_ok());
    let before = std::fs::read(&path).unwrap();
    super::known_hosts::trust_host_key_at(
        &cellar_core::SshHostKey {
            host: "db".into(),
            port: 2222,
            algorithm: public.algorithm().to_string(),
            fingerprint: public
                .fingerprint(keys::ssh_key::HashAlg::Sha256)
                .to_string(),
            public_key: public.to_openssh().unwrap(),
        },
        &path,
    )
    .unwrap();
    assert_eq!(before, std::fs::read(&path).unwrap());
    assert!(verify_host_key("other", 2222, &public, &path).is_err());
    assert!(verify_host_key("db", 2222, key(2).public_key(), &path).is_err());
}

#[tokio::test]
async fn unknown_jump_and_server_keys_fail_before_forwarding() {
    let target = echo().await;
    let server = start_server(
        5,
        MockSsh {
            user: "server",
            password: "server",
            public_key: None,
        },
    )
    .await;
    let jump = start_server(
        6,
        MockSsh {
            user: "jump",
            password: "jump",
            public_key: None,
        },
    )
    .await;
    let temp = tempfile::tempdir().unwrap();
    let known = temp.path().join("known_hosts");
    let mut config = config(
        host(server.port, "server"),
        ([127, 0, 0, 1], target.port).into(),
    );
    config.ssh.as_mut().unwrap().jump_host = Some(host(jump.port, "jump"));
    let secrets = ConnectionSecrets {
        ssh_password: Some("server".into()),
        jump_password: Some("jump".into()),
        ..Default::default()
    };
    // Both hops must preserve the typed challenge through russh authentication.
    for (port, seed) in [(jump.port, 6), (server.port, 5)] {
        let error = match SshTunnel::open_inner(&config, &secrets, &known, forward_bytes).await {
            Err(error) => CellarError::from(error),
            Ok(_) => panic!("unknown hop was accepted"),
        };
        let CellarError::SshHostKeyUnknown(challenge) = error else {
            panic!("typed challenge lost: {error}");
        };
        assert_eq!(challenge.host, "127.0.0.1");
        assert_eq!(challenge.port, port);
        assert_eq!(
            challenge.public_key,
            key(seed).public_key().to_openssh().unwrap()
        );
        super::known_hosts::trust_host_key_at(&challenge, &known).unwrap();
    }
    let tunnel = SshTunnel::open_inner(&config, &secrets, &known, forward_bytes)
        .await
        .unwrap();
    let _stream = roundtrip(&tunnel).await;
    tunnel.close().await;
    // Every new network attempt rechecks the exact recorded key, not a cached
    // confirmation or an algorithm-only pin.
    trust(&known, &[(&jump, 6), (&server, 9)]);
    let error = match SshTunnel::open_inner(&config, &secrets, &known, forward_bytes).await {
        Err(error) => CellarError::from(error),
        Ok(_) => panic!("changed server key was accepted"),
    };
    assert!(matches!(error, CellarError::Authentication(_)));
}

#[tokio::test]
async fn ssh_agent_signs_locally_without_forwarding_agent_to_server() {
    use futures::StreamExt;
    let client_key = key(8);
    let target = echo().await;
    let server = start_server(
        7,
        MockSsh {
            user: "agent-user",
            password: "unused",
            public_key: Some(client_key.public_key().clone()),
        },
    )
    .await;
    let temp = tempfile::tempdir().unwrap();
    let known = temp.path().join("known_hosts");
    trust(&known, &[(&server, 7)]);
    let mut session = client::connect(
        Arc::new(client::Config::default()),
        ("127.0.0.1", server.port),
        HostVerifier {
            host: "127.0.0.1".into(),
            port: server.port,
            known_hosts: known,
        },
    )
    .await
    .unwrap();
    let (client_stream, agent_stream) = tokio::io::duplex(8192);
    let incoming = futures::stream::iter([Ok(agent_stream)]).chain(futures::stream::pending());
    let agent_task = tokio::spawn(keys::agent::server::serve(incoming, ()));
    let mut agent = keys::agent::client::AgentClient::connect(client_stream);
    assert!(
        !authenticate_agent_with(&mut session, "agent-user", &mut agent)
            .await
            .unwrap()
    );
    agent.add_identity(&client_key, &[]).await.unwrap();
    assert!(
        authenticate_agent_with(&mut session, "agent-user", &mut agent)
            .await
            .unwrap()
    );
    let channel = session
        .channel_open_direct_tcpip("127.0.0.1", target.port.into(), "127.0.0.1", 0)
        .await
        .unwrap();
    let mut stream = channel.into_stream();
    stream.write_all(b"agent").await.unwrap();
    let mut reply = [0; 5];
    timeout(Duration::from_secs(5), stream.read_exact(&mut reply))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(&reply, b"agent");
    session
        .disconnect(russh::Disconnect::ByApplication, "test complete", "")
        .await
        .unwrap();
    agent_task.abort();
}

#[tokio::test]
async fn refused_forwarding_channel_reports_readable_error() {
    let server = start_server(
        10,
        MockSsh {
            user: "user",
            password: "password",
            public_key: None,
        },
    )
    .await;
    let temp = tempfile::tempdir().unwrap();
    let known = temp.path().join("known_hosts");
    trust(&known, &[(&server, 10)]);
    let mut config = config(host(server.port, "user"), ([127, 0, 0, 1], 5432).into());
    // The mock drops (rejects) channel requests for non-loopback targets.
    config.host = "blocked.invalid".into();
    let secrets = ConnectionSecrets {
        ssh_password: Some("password".into()),
        ..Default::default()
    };
    let tunnel = SshTunnel::open_inner(&config, &secrets, &known, forward_bytes)
        .await
        .unwrap();
    let mut local = TcpStream::connect(tunnel.address()).await.unwrap();
    let mut byte = [0];
    let closed = timeout(Duration::from_secs(5), local.read(&mut byte))
        .await
        .unwrap();
    assert!(matches!(closed, Ok(0) | Err(_)));
    let message = tunnel.take_error().unwrap().to_string();
    assert!(message.contains("administratively prohibited"), "{message}");
    assert!(
        !message.contains("Ok(") && !message.contains("Err("),
        "{message}"
    );
    tunnel.close().await;
}

#[test]
fn private_key_paths_expand_only_leading_home_references() {
    let home = || Some(PathBuf::from("/home/u"));
    let expand = |path: &str, windows: bool| expand_key_path(path, windows, home).unwrap();
    assert_eq!(expand("~", false), PathBuf::from("/home/u"));
    assert_eq!(expand("~/.ssh/id", false), PathBuf::from("/home/u/.ssh/id"));
    // Extra separators must not turn the remainder into an absolute path.
    assert_eq!(
        expand("~//etc/key", false),
        PathBuf::from("/home/u/etc/key")
    );
    for literal in [
        "~other/key",
        "/abs/~/key",
        "$HOME/key",
        "%USERPROFILE%\\key",
    ] {
        assert_eq!(expand(literal, false), PathBuf::from(literal));
    }
    assert_eq!(home_relative("~\\.ssh\\id", true), Some(".ssh\\id"));
    assert_eq!(
        home_relative("%userprofile%\\.ssh\\id", true),
        Some(".ssh\\id")
    );
    assert_eq!(home_relative("%USERPROFILE%", true), Some(""));
    assert_eq!(home_relative("%USERPROFILE%x", true), None);
    assert_eq!(home_relative("%APPDATA%\\key", true), None);
    assert!(expand_key_path("~/id", false, || None).is_err());
}

#[tokio::test(start_paused = true)]
async fn stalled_ssh_startup_times_out_and_closes_socket() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let (opened, open_done) = tokio::sync::oneshot::channel::<()>();
    let stalled = tokio::spawn(async move {
        tokio::select! {
            accepted = listener.accept() => {
                let (mut stream, _) = accepted.unwrap();
                let mut bytes = Vec::new();
                stream.read_to_end(&mut bytes).await.unwrap();
            }
            // With paused time the deadline can fire before the TCP connect
            // completes; then no socket exists and there is nothing to close.
            _ = open_done => {}
        }
    });
    let config = config(host(port, "user"), ([127, 0, 0, 1], 5432).into());
    let result = SshTunnel::open(&config, &ConnectionSecrets::default()).await;
    assert!(matches!(result, Err(CellarError::Timeout(_))));
    let _ = opened.send(());
    // If a connection was accepted, EOF proves cancellation shut the socket
    // down. With paused time the deadline may fire before russh schedules its
    // identification write, so the bytes read before EOF are not asserted.
    stalled.await.unwrap();
}
