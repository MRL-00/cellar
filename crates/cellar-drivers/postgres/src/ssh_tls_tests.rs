use super::*;
use cellar_core::driver::{Engine, SslMode};
use tokio::net::TcpListener;

fn config(host: &str, ssl_mode: SslMode) -> ConnectionConfig {
    ConnectionConfig {
        id: "tls-test".into(),
        name: "TLS test".into(),
        engine: Engine::Postgres,
        host: host.into(),
        port: 5432,
        database: "postgres".into(),
        user: "postgres".into(),
        ssl_mode,
        ssh: None,
        env_tag: None,
        application_name: None,
        color: None,
    }
}

async fn tls_roundtrip(host: &str, mode: SslMode, trust_certificate: bool) -> CellarResult<()> {
    let certified = rcgen::generate_simple_self_signed(vec!["database.internal".into()]).unwrap();
    let cert = certified.cert.pem();
    let options = tokio_rustls::rustls::ServerConfig::builder_with_provider(
        tokio_rustls::rustls::crypto::ring::default_provider().into(),
    )
    .with_safe_default_protocol_versions()
    .unwrap()
    .with_no_client_auth()
    .with_single_cert(
        vec![certified.cert.der().clone()],
        tokio_rustls::rustls::pki_types::PrivatePkcs8KeyDer::from(
            certified.key_pair.serialize_der(),
        )
        .into(),
    )
    .unwrap();
    let acceptor = tokio_rustls::TlsAcceptor::from(std::sync::Arc::new(options));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut request = [0; 8];
        stream.read_exact(&mut request).await.unwrap();
        assert_eq!(request, [0, 0, 0, 8, 4, 210, 22, 47]);
        stream.write_all(b"S").await.unwrap();
        if let Ok(mut tls) = acceptor.accept(stream).await {
            let mut data = [0; 4];
            if tls.read_exact(&mut data).await.is_ok() {
                tls.write_all(&data).await.unwrap();
            }
        }
    });
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("root.crt");
    std::fs::write(&path, cert).unwrap();
    let stream = tokio::net::TcpStream::connect(address).await.unwrap();
    let result = database_stream_inner(
        stream,
        &config(host, mode),
        trust_certificate.then_some(path),
    )
    .await;
    let result = match result {
        Ok(mut stream) => {
            stream.write_all(b"test").await.unwrap();
            let mut reply = [0; 4];
            stream.read_exact(&mut reply).await.unwrap();
            assert_eq!(&reply, b"test");
            Ok(())
        }
        Err(error) => Err(error),
    };
    server.await.unwrap();
    result
}

#[tokio::test]
async fn verify_full_validates_original_database_hostname_not_loopback() {
    tls_roundtrip("database.internal", SslMode::VerifyFull, true)
        .await
        .unwrap();
    assert!(matches!(
        tls_roundtrip("localhost", SslMode::VerifyFull, true).await,
        Err(CellarError::Tls(_))
    ));
    assert!(matches!(
        tls_roundtrip("database.internal", SslMode::VerifyFull, false).await,
        Err(CellarError::Tls(_))
    ));
}

#[tokio::test]
async fn verify_ca_checks_trust_without_hostname_and_require_allows_self_signed() {
    tls_roundtrip("other.internal", SslMode::VerifyCa, true)
        .await
        .unwrap();
    assert!(tls_roundtrip("other.internal", SslMode::VerifyCa, false)
        .await
        .is_err());
    tls_roundtrip("other.internal", SslMode::Require, false)
        .await
        .unwrap();
}

#[tokio::test]
async fn only_prefer_allows_server_to_decline_tls() {
    for (mode, accepted) in [
        (SslMode::Prefer, true),
        (SslMode::Require, false),
        (SslMode::VerifyCa, false),
        (SslMode::VerifyFull, false),
    ] {
        let (client, mut server) = tokio::io::duplex(64);
        let responder = tokio::spawn(async move {
            let mut request = [0; 8];
            server.read_exact(&mut request).await.unwrap();
            server.write_all(b"N").await.unwrap();
        });
        assert_eq!(
            database_stream_inner(client, &config("db", mode), None)
                .await
                .is_ok(),
            accepted
        );
        responder.await.unwrap();
    }
}
