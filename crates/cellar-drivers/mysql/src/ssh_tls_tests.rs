use super::*;
use cellar_core::driver::Engine;
use sqlx::{ConnectOptions, Connection};
use tokio::net::TcpListener;

fn config(mode: SslMode, host: &str) -> ConnectionConfig {
    ConnectionConfig {
        id: "test".into(),
        name: "test".into(),
        engine: Engine::PlanetScale,
        host: host.into(),
        port: 3306,
        database: "db".into(),
        user: "user".into(),
        ssl_mode: mode,
        ssh: None,
        env_tag: None,
        application_name: None,
        color: None,
    }
}

fn packet(sequence: u8, payload: &[u8]) -> Vec<u8> {
    let mut result = vec![
        payload.len() as u8,
        (payload.len() >> 8) as u8,
        (payload.len() >> 16) as u8,
        sequence,
    ];
    result.extend_from_slice(payload);
    result
}

fn greeting(tls: bool, plugin: &str) -> Vec<u8> {
    let capabilities: u32 = 0x0008_8209 | if tls { 0x800 } else { 0 };
    let mut body = b"\x0a8.0.36\0\x01\0\0\0abcdefgh\0".to_vec();
    body.extend_from_slice(&(capabilities as u16).to_le_bytes());
    body.push(45);
    body.extend_from_slice(&2u16.to_le_bytes());
    body.extend_from_slice(&((capabilities >> 16) as u16).to_le_bytes());
    body.push(21);
    body.extend_from_slice(&[0; 10]);
    body.extend_from_slice(b"ijklmnopqrst\0");
    body.extend_from_slice(plugin.as_bytes());
    body.push(0);
    packet(0, &body)
}

fn server_tls(host: &str) -> (TlsAcceptor, native_tls::Certificate) {
    let cert = rcgen::generate_simple_self_signed(vec![host.into()]).unwrap();
    let trust = native_tls::Certificate::from_der(cert.cert.der()).unwrap();
    let key = rustls::pki_types::PrivatePkcs8KeyDer::from(cert.key_pair.serialize_der());
    let server = rustls::ServerConfig::builder_with_provider(std::sync::Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .with_no_client_auth()
    .with_single_cert(vec![cert.cert.der().clone()], key.into())
    .unwrap();
    (TlsAcceptor::from(std::sync::Arc::new(server)), trust)
}

// Actual production forward callback + actual SQLx login. Only the SSH channel
// itself is replaced by duplex IO; SSH trust/auth are tested by cellar-ssh.
async fn login(mode: SslMode, hostname: &str, plugin: &str, tls: bool) -> (bool, CellarResult<()>) {
    login_with_trust(mode, hostname, plugin, tls, true, true).await
}

async fn login_with_trust(
    mode: SslMode,
    hostname: &str,
    plugin: &str,
    tls: bool,
    trust_remote: bool,
    trust_local: bool,
) -> (bool, CellarResult<()>) {
    login_case(
        mode,
        hostname,
        plugin,
        tls,
        (trust_remote, trust_local),
        Some("secret"),
    )
    .await
}

async fn login_case(
    mode: SslMode,
    hostname: &str,
    plugin: &str,
    tls: bool,
    trust: (bool, bool),
    password: Option<&str>,
) -> (bool, CellarResult<()>) {
    let (trust_remote, trust_local) = trust;
    let (acceptor, trust) = server_tls("database.example");
    let mut connector = native_tls::TlsConnector::builder();
    configure_remote(&mut connector, mode);
    if trust_remote {
        connector.add_root_certificate(trust);
    }
    let bridge =
        TlsBridge::with_connector(&config(mode, hostname), connector.build().unwrap()).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let mut options = bridge.local_options(
        sqlx::mysql::MySqlConnectOptions::new()
            .host("127.0.0.1")
            .port(address.port())
            .username("user")
            .database("db"),
    );
    if let Some(password) = password {
        options = options.password(password);
    }
    let password_configured = password.is_some();
    let password = password.unwrap_or_default().to_owned();
    if !trust_local {
        options = options.ssl_ca_from_pem(
            rcgen::generate_simple_self_signed(vec!["127.0.0.1".into()])
                .unwrap()
                .cert
                .pem()
                .into_bytes(),
        );
    }
    let (channel, mut server) = tokio::io::duplex(64 * 1024);
    let forward = tokio::spawn(async move {
        let (local, _) = listener.accept().await.unwrap();
        bridge.forward(local, Box::new(channel)).await
    });
    let plugin = plugin.to_owned();
    let mock = tokio::spawn(async move {
        let initial_plugin = if plugin == "switch_sha256_password" {
            "mysql_native_password"
        } else {
            &plugin
        };
        server
            .write_all(&greeting(tls, initial_plugin))
            .await
            .unwrap();
        let Ok(first) = read_packet(&mut server).await else {
            return false;
        };
        let mut server: BoxedStream = if tls && mode != SslMode::Disable {
            assert_eq!(first[3], 1);
            assert_eq!(first.len(), 36, "SSLRequest, not login");
            let Ok(server) = acceptor.accept(server).await else {
                return false;
            };
            Box::new(server)
        } else {
            // Plain fallback sends login at sequence 1, without SSLRequest.
            assert_eq!(first[3], 1);
            Box::new(server)
        };
        let sequence = if tls && mode != SslMode::Disable {
            let Ok(response) = read_packet(&mut server).await else {
                return false;
            };
            assert_eq!(response[3], 2, "login sequence unchanged by TLS bridge");
            if plugin == "sha256_password" {
                assert!(response
                    .windows(b"secret\0".len())
                    .any(|w| w == b"secret\0"));
            } else {
                server.write_all(&packet(3, &[1, 4])).await.unwrap();
                let full = read_packet(&mut server).await.unwrap();
                assert_eq!(
                    full,
                    packet(4, b"secret\0"),
                    "TLS-aware full authentication"
                );
            }
            if plugin == "sha256_password" {
                3
            } else {
                5
            }
        } else if plugin == "sha256_password" && !password_configured {
            assert_eq!(first[3], 1);
            assert_eq!(
                first[41], 0,
                "absent password keeps empty auth, no RSA request"
            );
            2
        } else if mode != SslMode::Disable && plugin != "mysql_native_password" {
            assert_eq!(first[3], 1, "remote login has no SSLRequest sequence");
            assert_eq!(first[5] & 8, 0, "remote login must clear CLIENT_SSL");
            assert!(
                !first.windows(7).any(|w| w == b"secret\0"),
                "never forward plaintext password"
            );
            let next = if plugin == "switch_sha256_password" {
                let switch = b"\xfesha256_password\0abcdefghijklmnopqrst\0";
                server.write_all(&packet(2, switch)).await.unwrap();
                assert_eq!(read_packet(&mut server).await.unwrap(), packet(3, &[1]));
                4
            } else if plugin == "caching_sha2_password" {
                server.write_all(&packet(2, &[1, 4])).await.unwrap();
                assert_eq!(read_packet(&mut server).await.unwrap(), packet(3, &[2]));
                4
            } else {
                // HandshakeResponse has auth length 1 and public-key request 1.
                assert_eq!(&first[41..43], &[1, 1]);
                2
            };
            verify_rsa(&mut server, next, password.as_bytes()).await;
            next + 2
        } else {
            2
        };
        server
            .write_all(&packet(sequence, &[0, 0, 0, 2, 0, 0, 0]))
            .await
            .unwrap();
        // SQLx's connection initialization is a query, sequence resets to zero.
        let init = read_packet(&mut server).await.unwrap();
        assert_eq!(init[3], 0);
        assert_eq!(init[4], 3);
        server
            .write_all(&packet(1, &[0, 0, 0, 2, 0, 0, 0]))
            .await
            .unwrap();
        // Keep the server open until SQLx closes it.
        let _ = read_packet(&mut server).await;
        true
    });
    let connected = match options.connect().await {
        Ok(connection) => {
            connection.close().await.unwrap();
            true
        }
        Err(_) => false,
    };
    let result = forward.await.unwrap();
    assert_eq!(mock.await.unwrap(), connected);
    (connected, result)
}

async fn verify_rsa(server: &mut BoxedStream, sequence: u8, expected: &[u8]) {
    use rsa::pkcs8::{EncodePublicKey, LineEnding};
    let key = rsa::RsaPrivateKey::new(&mut rand::thread_rng(), 2048).unwrap();
    let pem = key
        .to_public_key()
        .to_public_key_pem(LineEnding::LF)
        .unwrap();
    let mut response = vec![1];
    response.extend_from_slice(pem.as_bytes());
    server
        .write_all(&packet(sequence, &response))
        .await
        .unwrap();
    let encrypted = read_packet(server).await.unwrap();
    assert_eq!(encrypted[3], sequence + 1);
    let decrypted = key
        .decrypt(rsa::Oaep::new::<sha1::Sha1>(), &encrypted[4..])
        .unwrap();
    let password: Vec<_> = decrypted
        .iter()
        .enumerate()
        .map(|(i, b)| b ^ b"abcdefghijklmnopqrst"[i % 20])
        .collect();
    assert_eq!(password.strip_suffix(&[0]).unwrap(), expected);
}

#[tokio::test]
async fn preferred_plaintext_rsa_full_authentication() {
    for plugin in [
        "caching_sha2_password",
        "sha256_password",
        "switch_sha256_password",
    ] {
        let (connected, result) = login(SslMode::Prefer, "database.example", plugin, false).await;
        assert!(connected);
        result.unwrap();
    }
}

#[tokio::test]
async fn passwordless_sha256_initial_and_switch_authentication() {
    for (plugin, password) in [
        ("sha256_password", None),
        ("switch_sha256_password", None),
        ("sha256_password", Some("")),
    ] {
        let (connected, result) = login_case(
            SslMode::Prefer,
            "database.example",
            plugin,
            false,
            (true, true),
            password,
        )
        .await;
        assert!(connected);
        result.unwrap();
    }
}

#[tokio::test]
async fn stalled_handshake_times_out_and_drops_both_streams() {
    let bridge = TlsBridge::new(&config(SslMode::Prefer, "database.example")).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let (client, accepted) = tokio::join!(
        TcpStream::connect(listener.local_addr().unwrap()),
        listener.accept()
    );
    let mut client = client.unwrap();
    let local = accepted.unwrap().0;
    let (channel, mut server) = tokio::io::duplex(64);
    tokio::time::pause();
    let forward = tokio::spawn(async move { bridge.forward(local, Box::new(channel)).await });
    tokio::task::yield_now().await;
    tokio::time::advance(std::time::Duration::from_secs(31)).await;
    assert!(matches!(
        forward.await.unwrap(),
        Err(CellarError::Timeout(_))
    ));
    let mut byte = [0];
    assert_eq!(client.read(&mut byte).await.unwrap(), 0);
    assert_eq!(server.read(&mut byte).await.unwrap(), 0);
}

#[tokio::test]
async fn original_hostname_and_full_authentication() {
    for plugin in ["caching_sha2_password", "sha256_password"] {
        let (connected, result) =
            login(SslMode::VerifyFull, "database.example", plugin, true).await;
        assert!(connected);
        result.unwrap();
    }
}

#[tokio::test]
async fn wrong_remote_hostname_fails_before_login() {
    let (connected, result) = login(
        SslMode::VerifyFull,
        "wrong.example",
        "caching_sha2_password",
        true,
    )
    .await;
    assert!(!connected);
    assert!(matches!(result, Err(CellarError::Tls(_))));
}

#[tokio::test]
async fn verify_ca_does_not_require_hostname() {
    assert!(
        login(
            SslMode::VerifyCa,
            "wrong.example",
            "caching_sha2_password",
            true
        )
        .await
        .0
    );
}

#[tokio::test]
async fn preferred_and_required_tls_authentication() {
    for mode in [SslMode::Prefer, SslMode::Require] {
        assert!(
            login(mode, "database.example", "caching_sha2_password", true)
                .await
                .0
        );
    }
}

#[tokio::test]
async fn preferred_plaintext_fallback_and_required_rejection() {
    assert!(
        login(
            SslMode::Prefer,
            "database.example",
            "mysql_native_password",
            false
        )
        .await
        .0
    );
    let (connected, result) = login(
        SslMode::Require,
        "database.example",
        "mysql_native_password",
        false,
    )
    .await;
    assert!(!connected);
    assert!(matches!(result, Err(CellarError::Tls(_))));
}

#[tokio::test]
async fn certificate_trust_is_enforced_on_both_legs() {
    for mode in [SslMode::VerifyCa, SslMode::VerifyFull] {
        let (connected, result) = login_with_trust(
            mode,
            "database.example",
            "caching_sha2_password",
            true,
            false,
            true,
        )
        .await;
        assert!(!connected);
        assert!(matches!(result, Err(CellarError::Tls(_))));
    }
    // Even Preferred must validate the local certificate and loopback SAN.
    for tls in [false, true] {
        let (connected, result) = login_with_trust(
            SslMode::Prefer,
            "database.example",
            "caching_sha2_password",
            tls,
            true,
            false,
        )
        .await;
        assert!(!connected);
        assert!(matches!(result, Err(CellarError::Tls(_))));
    }
}

#[tokio::test]
async fn unknown_plaintext_authentication_fails_closed() {
    let (connected, result) =
        login(SslMode::Prefer, "database.example", "unknown_plugin", false).await;
    assert!(!connected);
    assert!(matches!(result, Err(CellarError::Authentication(_))));
}

#[tokio::test]
async fn preferred_never_retries_failed_tls_in_plaintext() {
    let bridge = TlsBridge::new(&config(SslMode::Prefer, "database.example")).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let options = bridge.local_options(
        sqlx::mysql::MySqlConnectOptions::new()
            .host("127.0.0.1")
            .port(listener.local_addr().unwrap().port())
            .password("secret"),
    );
    let (channel, mut server) = tokio::io::duplex(4096);
    let forward = tokio::spawn(async move {
        let (local, _) = listener.accept().await.unwrap();
        bridge.forward(local, Box::new(channel)).await
    });
    let mock = tokio::spawn(async move {
        server
            .write_all(&greeting(true, "caching_sha2_password"))
            .await
            .unwrap();
        let request = read_packet(&mut server).await.unwrap();
        assert_eq!(request.len(), 36);
        assert_eq!(request[3], 1);
        // Fatal TLS handshake_failure alert; never supply a successful TLS leg.
        server.write_all(&[21, 3, 3, 0, 2, 2, 40]).await.unwrap();
    });
    assert!(options.connect().await.is_err());
    mock.await.unwrap();
    assert!(matches!(forward.await.unwrap(), Err(CellarError::Tls(_))));
}

#[tokio::test]
async fn truncated_packet_is_rejected() {
    let (mut writer, mut reader) = tokio::io::duplex(64);
    writer.write_all(&[32, 0, 0, 1, 1, 2]).await.unwrap();
    drop(writer);
    assert!(read_packet(&mut reader).await.is_err());
    for body in [&[][..], &[10][..], b"\x0a8.0\0"] {
        assert!(greeting_flags(&packet(0, body)).is_err());
    }
    let complete = greeting(true, "caching_sha2_password");
    for end in 4..complete.len() {
        let truncated = packet(0, &complete[4..end]);
        if let Ok(flags) = greeting_flags(&truncated) {
            assert!(plain_auth::Handshake::parse(&truncated, flags).is_err());
        }
    }
}

// Real SQLx with the cleartext plugin enabled; the bridge must relay the switch
// over a TLS remote leg and refuse it, before SQLx answers, over plaintext.
async fn clear_password_switch(tls: bool) -> (bool, CellarResult<()>, bool) {
    let (acceptor, trust) = server_tls("database.example");
    let mut connector = native_tls::TlsConnector::builder();
    configure_remote(&mut connector, SslMode::Prefer);
    connector.add_root_certificate(trust);
    let bridge = TlsBridge::with_connector(
        &config(SslMode::Prefer, "database.example"),
        connector.build().unwrap(),
    )
    .unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let options = bridge
        .local_options(
            sqlx::mysql::MySqlConnectOptions::new()
                .host("127.0.0.1")
                .port(listener.local_addr().unwrap().port())
                .username("user")
                .password("secret"),
        )
        .enable_cleartext_plugin(true);
    let (channel, mut server) = tokio::io::duplex(64 * 1024);
    let forward = tokio::spawn(async move {
        let (local, _) = listener.accept().await.unwrap();
        bridge.forward(local, Box::new(channel)).await
    });
    let mock = tokio::spawn(async move {
        server
            .write_all(&greeting(tls, "caching_sha2_password"))
            .await
            .unwrap();
        let first = read_packet(&mut server).await.unwrap();
        let (mut server, sequence): (BoxedStream, u8) = if tls {
            assert_eq!(first.len(), 36, "SSLRequest, not login");
            let mut server = acceptor.accept(server).await.unwrap();
            assert_eq!(read_packet(&mut server).await.unwrap()[3], 2);
            (Box::new(server), 3)
        } else {
            (Box::new(server), 2)
        };
        // AWS Aurora IAM style: plugin name without challenge data.
        let switch = b"\xfemysql_clear_password\0";
        server.write_all(&packet(sequence, switch)).await.unwrap();
        let Ok(answer) = read_packet(&mut server).await else {
            return false;
        };
        assert!(tls, "cleartext password reached a plaintext server");
        assert_eq!(answer, packet(sequence + 1, b"secret\0"));
        let ok = [0, 0, 0, 2, 0, 0, 0];
        server.write_all(&packet(sequence + 2, &ok)).await.unwrap();
        assert_eq!(read_packet(&mut server).await.unwrap()[3], 0);
        server.write_all(&packet(1, &ok)).await.unwrap();
        let _ = read_packet(&mut server).await;
        true
    });
    let connected = match options.connect().await {
        Ok(connection) => {
            connection.close().await.unwrap();
            true
        }
        Err(_) => false,
    };
    (connected, forward.await.unwrap(), mock.await.unwrap())
}

#[tokio::test]
async fn clear_password_relayed_only_over_remote_tls() {
    let (connected, result, answered) = clear_password_switch(true).await;
    assert!(connected && answered);
    result.unwrap();
    let (connected, result, answered) = clear_password_switch(false).await;
    assert!(!connected && !answered);
    assert!(matches!(result, Err(CellarError::Authentication(_))));
}
