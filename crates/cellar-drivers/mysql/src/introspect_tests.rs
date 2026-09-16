//! MySQL-backed metadata decoding regression tests. Run with:
//!
//!     cargo test -p cellar-driver-mysql --features integration-tests

use super::*;
use sqlx::mysql::MySqlConnectOptions;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::mysql::Mysql;

#[tokio::test]
async fn metadata_text_accepts_binary_catalog_fields() {
    let container = Mysql::default()
        .start()
        .await
        .expect("docker is required for integration tests");
    let host = container
        .get_host()
        .await
        .expect("container host")
        .to_string();
    let port = container
        .get_host_port_ipv4(3306)
        .await
        .expect("container port");
    let options = MySqlConnectOptions::new()
        .host(&host)
        .port(port)
        .username("root");
    let pool = MySqlPool::connect_with(options)
        .await
        .expect("connect to test MySQL");
    let row = sqlx::query(
        "SELECT CAST('orders' AS BINARY) AS TABLE_NAME, \
                CAST('café_日本' AS BINARY) AS COLUMN_NAME, \
                CAST('' AS BINARY) AS COLUMN_DEFAULT, \
                CAST(NULL AS BINARY) AS VIEW_DEFINITION, \
                CAST('ordinary' AS CHAR CHARACTER SET utf8mb4) AS TEXT_VALUE, \
                X'FF' AS INVALID_UTF8",
    )
    .fetch_one(&pool)
    .await
    .expect("fetch metadata fixture");

    // Prove the fixture reproduces the original checked-decoding failure.
    assert!(row.try_get::<String, _>("TABLE_NAME").is_err());
    assert_eq!(metadata_text(&row, "TABLE_NAME").unwrap(), "orders");
    assert_eq!(metadata_text(&row, "COLUMN_NAME").unwrap(), "café_日本");
    assert_eq!(metadata_text(&row, "TEXT_VALUE").unwrap(), "ordinary");
    assert_eq!(
        optional_metadata_text(&row, "TABLE_NAME").unwrap(),
        Some("orders".into())
    );
    assert_eq!(
        optional_metadata_text(&row, "TEXT_VALUE").unwrap(),
        Some("ordinary".into())
    );
    assert_eq!(
        optional_metadata_text(&row, "COLUMN_DEFAULT").unwrap(),
        Some(String::new())
    );
    assert_eq!(
        optional_metadata_text(&row, "VIEW_DEFINITION").unwrap(),
        None
    );
    assert!(metadata_text(&row, "VIEW_DEFINITION").is_err());
    assert!(metadata_text(&row, "INVALID_UTF8").is_err());
    assert!(optional_metadata_text(&row, "INVALID_UTF8").is_err());
    assert!(metadata_text(&row, "MISSING_COLUMN").is_err());
    pool.close().await;
}
