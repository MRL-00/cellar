//! MySQL-backed metadata decoding regression tests. Run with:
//!
//!     cargo test -p cellar-driver-mysql --features integration-tests

use super::*;
use crate::MySqlDriver;
use cellar_core::driver::{ConnectionConfig, Driver, Engine, SslMode};
use sqlx::mysql::MySqlConnectOptions;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::mysql::Mysql;

#[tokio::test]
async fn introspect_returns_columns_keys_indexes_and_views() {
    let container = Mysql::default()
        .with_init_sql(
            "CREATE TABLE customers (id INT PRIMARY KEY, email VARCHAR(255) UNIQUE); \
             CREATE TABLE orders (id INT PRIMARY KEY, customer_id INT NOT NULL, \
                 total INT DEFAULT 0, \
                 CONSTRAINT orders_customer_fk FOREIGN KEY (customer_id) REFERENCES customers(id)); \
             CREATE INDEX orders_total_idx ON orders(total); \
             CREATE VIEW order_totals AS SELECT id, total FROM orders;"
                .as_bytes()
                .to_vec(),
        )
        .start()
        .await
        .expect("docker is required for integration tests");
    let config = ConnectionConfig {
        id: "test".into(),
        name: "test".into(),
        engine: Engine::MySql,
        host: container
            .get_host()
            .await
            .expect("container host")
            .to_string(),
        port: container
            .get_host_port_ipv4(3306)
            .await
            .expect("container port"),
        database: "test".into(),
        user: "root".into(),
        ssl_mode: SslMode::Disable,
        env_tag: None,
        application_name: None,
        color: None,
    };
    let driver = MySqlDriver;
    let conn = driver.connect(&config, None).await.expect("connect");
    let databases = driver.introspect(conn.as_ref()).await.expect("introspect");
    assert_eq!(databases.len(), 1);
    assert_eq!(databases[0].name, "test");
    let schema = &databases[0].schemas[0];
    assert_eq!(schema.tables.len(), 2);
    let orders = schema
        .tables
        .iter()
        .find(|t| t.name == "orders")
        .expect("orders");
    assert_eq!(
        orders
            .columns
            .iter()
            .map(|c| (c.name.as_str(), c.ordinal))
            .collect::<Vec<_>>(),
        vec![("id", 1), ("customer_id", 2), ("total", 3)]
    );
    assert_eq!(orders.primary_key, vec!["id"]);
    assert!(orders.columns[0].is_primary_key);
    assert!(!orders.columns[1].nullable);
    assert_eq!(orders.columns[2].default.as_deref(), Some("0"));
    let fk = &orders.foreign_keys[0];
    assert_eq!(fk.columns, vec!["customer_id"]);
    assert_eq!(fk.referenced_table, "customers");
    assert_eq!(fk.referenced_columns, vec!["id"]);
    let primary = orders
        .indexes
        .iter()
        .find(|i| i.primary)
        .expect("primary index");
    assert!(primary.unique);
    let index = orders
        .indexes
        .iter()
        .find(|i| i.name == "orders_total_idx")
        .expect("total index");
    assert!(!index.unique);
    assert_eq!(index.columns, vec!["total"]);
    let customers = schema
        .tables
        .iter()
        .find(|t| t.name == "customers")
        .expect("customers");
    assert!(customers
        .indexes
        .iter()
        .any(|i| i.unique && !i.primary && i.columns == vec!["email"]));
    assert_eq!(schema.views.len(), 1);
    assert_eq!(schema.views[0].name, "order_totals");
    assert_eq!(schema.views[0].columns.len(), 2);
    assert!(schema.views[0].definition.is_some());
    conn.close().await.expect("close");
}

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
