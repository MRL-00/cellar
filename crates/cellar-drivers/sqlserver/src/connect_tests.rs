use super::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn config() -> ConnectionConfig {
    ConnectionConfig {
        id: "test".into(),
        name: "test".into(),
        engine: Engine::Mssql,
        host: "original.database.windows.net".into(),
        port: 1433,
        database: "test".into(),
        user: "test".into(),
        ssl_mode: SslMode::Disable,
        env_tag: None,
        application_name: None,
        color: None,
        ssh: None,
    }
}

#[test]
fn azure_requires_tls_and_redirect_fails_closed() {
    assert!(matches!(
        build_config(&config(), None, Engine::Azure),
        Err(CellarError::InvalidConfig(_))
    ));
    let mut config = config();
    config.ssl_mode = SslMode::VerifyFull;
    let tds = build_config(&config, None, Engine::Azure).unwrap();
    assert_eq!(tds.get_addr(), "original.database.windows.net:1433");
    let error = map_tiberius_connect_err(tiberius::error::Error::Routing {
        host: "redirect.database.windows.net".into(),
        port: 11000,
    });
    assert!(error.to_string().contains("redirects are not followed"));
}

#[tokio::test]
async fn close_does_not_wait_for_in_flight_operation() {
    let connection = SqlServerConnection {
        info: DriverInfo {
            engine: Engine::Mssql,
            version: String::new(),
        },
        config: config(),
        client: Mutex::new(None),
        closed: AtomicBool::new(false),
        tunnel: None,
    };
    // Simulates `with_client` holding the session for a long query.
    let busy = connection.client.lock().await;
    tokio::time::timeout(std::time::Duration::from_secs(1), connection.close())
        .await
        .expect("close blocked behind the client lock")
        .unwrap();
    drop(busy);
    let refused = connection.with_client(async |_| Ok(())).await;
    assert!(matches!(refused, Err(CellarError::NotConnected(_))));
}

async fn packet(stream: &mut TcpStream) -> (u8, Vec<u8>) {
    let mut header = [0; 8];
    stream.read_exact(&mut header).await.unwrap();
    let length = u16::from_be_bytes([header[2], header[3]]) as usize;
    let mut body = vec![0; length - 8];
    stream.read_exact(&mut body).await.unwrap();
    (header[0], body)
}

#[tokio::test]
async fn supplied_loopback_stream_keeps_original_host_in_tds_login() {
    let config = config();
    let tds = build_config(&config, Some("test-password"), Engine::Mssql).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut tcp, _) = listener.accept().await.unwrap();
        assert_eq!(packet(&mut tcp).await.0, 0x12); // PRELOGIN
                                                    // TDS response with ENCRYPT_NOT_SUP, matching this plaintext test.
        tcp.write_all(&[4, 1, 0, 15, 0, 0, 1, 0, 1, 0, 6, 0, 1, 255, 2])
            .await
            .unwrap();
        let (kind, login) = packet(&mut tcp).await;
        assert_eq!(kind, 0x10); // LOGIN7
        let host_offset = u16::from_le_bytes([login[52], login[53]]) as usize;
        let host_chars = u16::from_le_bytes([login[54], login[55]]) as usize;
        let host: Vec<u16> = login[host_offset..host_offset + host_chars * 2]
            .chunks_exact(2)
            .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
            .collect();
        String::from_utf16(&host).unwrap()
        // Drop without replying: this test inspects login, not a mock SQL engine.
    });
    let tcp = TcpStream::connect(address).await.unwrap();
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        Client::connect(tds, tcp.compat_write()),
    )
    .await
    .unwrap();
    assert!(result.is_err());
    assert_eq!(server.await.unwrap(), config.host);
}
