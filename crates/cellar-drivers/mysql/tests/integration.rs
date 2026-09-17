//! End-to-end MySQL driver tests using a disposable container. Run with:
//!
//!     cargo test -p cellar-driver-mysql --features integration-tests

#![cfg(feature = "integration-tests")]

use cellar_core::driver::{ConnectionConfig, Driver, Engine, SslMode};
use cellar_core::query::{Query, TableBrowseRequest};
use cellar_core::value::CellValue;
use cellar_driver_mysql::{browse_table, MySqlDriver};
use chrono::NaiveDate;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::mysql::Mysql;

#[tokio::test]
async fn browses_and_queries_timestamp_rows_with_bigint_keys() {
    let container = Mysql::default()
        .with_init_sql(
            "SET time_zone = '+00:00'; \
             CREATE TABLE events (id BIGINT PRIMARY KEY, unsigned_id BIGINT UNSIGNED, \
                 happened_at TIMESTAMP(6) NULL, local_time DATETIME(6) NULL, \
                 event_date DATE NULL, event_time TIME(6) NULL); \
             INSERT INTO events VALUES \
                 (1, 18446744073709551615, '2026-09-16 12:34:56.123456', \
                  '2026-09-16 12:34:56.123456', '2026-09-16', '12:34:56.123456'), \
                 (2147483648, 2147483648, '2026-09-17 00:00:00', \
                  '2026-09-17 00:00:00', '2026-09-17', '00:00:00'), \
                 (9223372036854775807, NULL, NULL, NULL, NULL, NULL);"
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
    let table = &databases[0].schemas[0].tables[0];
    assert_eq!(table.primary_key, vec!["id"]);
    let request = TableBrowseRequest {
        connection_id: config.id.clone(),
        database: Some(config.database.clone()),
        schema: "test".into(),
        table: "events".into(),
        limit: Some(10),
        offset: None,
        sorts: vec![],
        filters: vec![],
        primary_key_fallback_ordering: true,
        include_total: true,
    };
    let date = NaiveDate::from_ymd_opt(2026, 9, 16).unwrap();
    let timestamp = date.and_hms_micro_opt(12, 34, 56, 123456).unwrap();
    let midnight = date.succ_opt().unwrap().and_hms_opt(0, 0, 0).unwrap();
    let expected = vec![
        vec![
            CellValue::Int(1),
            CellValue::Numeric(u64::MAX.to_string()),
            CellValue::Timestamp(timestamp),
            CellValue::Timestamp(timestamp),
            CellValue::Date(date),
            CellValue::Time(timestamp.time()),
        ],
        vec![
            CellValue::Int(2147483648),
            CellValue::Int(2147483648),
            CellValue::Timestamp(midnight),
            CellValue::Timestamp(midnight),
            CellValue::Date(midnight.date()),
            CellValue::Time(midnight.time()),
        ],
        vec![
            CellValue::Int(i64::MAX),
            CellValue::Null,
            CellValue::Null,
            CellValue::Null,
            CellValue::Null,
            CellValue::Null,
        ],
    ];
    let result = browse_table(conn.as_ref(), &request, table)
        .await
        .expect("browse table");
    assert_eq!(result.total_rows, Some(3));
    assert_eq!(result.rows, expected);
    let query = Query::new("SELECT * FROM events ORDER BY id");
    let result = driver
        .execute_query(conn.as_ref(), &query)
        .await
        .expect("query");
    assert_eq!(result.rows, expected);
    let mut pages = Vec::new();
    driver
        .execute_query_stream(conn.as_ref(), &query, 1, &mut |page| {
            pages.push(page);
            Ok(())
        })
        .await
        .expect("stream query");
    assert_eq!(pages.len(), 3);
    assert_eq!(
        pages.into_iter().flat_map(|p| p.rows).collect::<Vec<_>>(),
        expected
    );
    conn.close().await.expect("close");
}
