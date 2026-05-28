//! End-to-end tests for the Postgres driver against a real Postgres 15
//! container. These only build under the `integration-tests` feature so a
//! plain `cargo test --workspace` on a developer laptop without Docker
//! stays green.
//!
//! Run with:
//!     cargo test -p cellar-driver-postgres --features integration-tests

#![cfg(feature = "integration-tests")]

use cellar_core::driver::{ConnectionConfig, Driver, Engine, SslMode};
use cellar_core::query::Query;
use cellar_core::value::CellValue;
use cellar_driver_postgres::PostgresDriver;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;

const SEED_STATEMENTS: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS customers (\
        id BIGSERIAL PRIMARY KEY, \
        email TEXT NOT NULL UNIQUE)",
    "CREATE TABLE IF NOT EXISTS orders (\
        id BIGSERIAL PRIMARY KEY, \
        customer_id BIGINT NOT NULL REFERENCES customers (id), \
        placed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), \
        total NUMERIC(12, 2) NOT NULL)",
    "CREATE INDEX IF NOT EXISTS orders_customer_idx ON orders (customer_id)",
];

struct Live {
    _container: testcontainers::ContainerAsync<Postgres>,
    config: ConnectionConfig,
    password: String,
}

async fn boot() -> Live {
    let image = Postgres::default();
    let container = image
        .start()
        .await
        .expect("docker is required for integration tests");
    let host = container
        .get_host()
        .await
        .expect("container host")
        .to_string();
    let port = container
        .get_host_port_ipv4(5432)
        .await
        .expect("container port");

    let config = ConnectionConfig {
        id: "test".into(),
        name: "test".into(),
        engine: Engine::Postgres,
        host,
        port,
        database: "postgres".into(),
        user: "postgres".into(),
        ssl_mode: SslMode::Disable,
        env_tag: None,
        application_name: Some("cellar-tests".into()),
        color: None,
    };

    let driver = PostgresDriver::new();
    let conn = driver
        .connect(&config, Some("postgres"))
        .await
        .expect("connect to seeded postgres");
    for stmt in SEED_STATEMENTS {
        driver
            .execute_query(conn.as_ref(), &Query::new(*stmt))
            .await
            .expect("seed statement");
    }

    Live {
        _container: container,
        config,
        password: "postgres".into(),
    }
}

#[tokio::test]
async fn connects_and_reports_version() {
    let live = boot().await;
    let driver = PostgresDriver::new();
    let conn = driver
        .connect(&live.config, Some(&live.password))
        .await
        .expect("connect");
    let info = conn.info();
    assert_eq!(info.engine, Engine::Postgres);
    assert!(info.version.to_lowercase().contains("postgresql"));
}

#[tokio::test]
async fn introspect_returns_seed_tables_with_keys() {
    let live = boot().await;
    let driver = PostgresDriver::new();
    let conn = driver
        .connect(&live.config, Some(&live.password))
        .await
        .expect("connect");
    let dbs = driver.introspect(conn.as_ref()).await.expect("introspect");
    let db = dbs
        .iter()
        .find(|d| d.name == "postgres")
        .expect("connected database in tree");
    assert!(db.is_default);
    let public = db
        .schemas
        .iter()
        .find(|s| s.name == "public")
        .expect("public schema");
    let customers = public
        .tables
        .iter()
        .find(|t| t.name == "customers")
        .expect("customers table");
    assert!(customers.primary_key.contains(&"id".to_string()));
    let orders = public
        .tables
        .iter()
        .find(|t| t.name == "orders")
        .expect("orders table");
    let fk = orders
        .foreign_keys
        .iter()
        .find(|fk| fk.columns == vec!["customer_id".to_string()])
        .expect("fk to customers");
    assert_eq!(fk.referenced_table, "customers");
    assert_eq!(fk.referenced_columns, vec!["id".to_string()]);
}

#[tokio::test]
async fn introspect_lists_every_database_on_the_server() {
    let live = boot().await;
    let driver = PostgresDriver::new();
    let conn = driver
        .connect(&live.config, Some(&live.password))
        .await
        .expect("connect");

    // A second database on the same server should appear in the tree even
    // though our connection is bound to `postgres`.
    driver
        .execute_query(conn.as_ref(), &Query::new("CREATE DATABASE extra_db"))
        .await
        .expect("create database");

    let dbs = driver.introspect(conn.as_ref()).await.expect("introspect");
    let names: Vec<&str> = dbs.iter().map(|d| d.name.as_str()).collect();
    assert!(names.contains(&"postgres"), "got {names:?}");
    assert!(names.contains(&"extra_db"), "got {names:?}");

    // Querying the sibling database routes to its own pool.
    let result = driver
        .execute_query(
            conn.as_ref(),
            &Query::new("SELECT current_database() AS d").with_database("extra_db"),
        )
        .await
        .expect("query sibling db");
    if let CellValue::Text(d) = &result.rows[0][0] {
        assert_eq!(d, "extra_db");
    } else {
        panic!("expected current_database text");
    }
}

#[tokio::test]
async fn execute_query_decodes_common_types() {
    let live = boot().await;
    let driver = PostgresDriver::new();
    let conn = driver
        .connect(&live.config, Some(&live.password))
        .await
        .expect("connect");

    let result = driver
        .execute_query(
            conn.as_ref(),
            &Query::new(
                "SELECT \
                   1::int4 AS i, \
                   2.5::float8 AS f, \
                   'hello'::text AS t, \
                   NULL::text AS n, \
                   true AS b, \
                   '{\"k\":1}'::jsonb AS j, \
                   '11111111-1111-1111-1111-111111111111'::uuid AS u, \
                   '2026-05-28'::date AS d, \
                   '2026-05-28 10:00:00+00'::timestamptz AS ts, \
                   '123456789012345.6789'::numeric AS num",
            ),
        )
        .await
        .expect("query");

    assert_eq!(result.columns.len(), 10);
    assert_eq!(result.rows.len(), 1);
    let row = &result.rows[0];

    assert!(matches!(row[0], CellValue::Int(1)));
    assert!(matches!(row[1], CellValue::Float(_)));
    if let CellValue::Text(t) = &row[2] {
        assert_eq!(t, "hello");
    } else {
        panic!("expected text");
    }
    assert!(matches!(row[3], CellValue::Null));
    assert!(matches!(row[4], CellValue::Bool(true)));
    assert!(matches!(row[5], CellValue::Json(_)));
    assert!(matches!(row[6], CellValue::Uuid(_)));
    assert!(matches!(row[7], CellValue::Date(_)));
    assert!(matches!(row[8], CellValue::TimestampTz(_)));
    if let CellValue::Numeric(n) = &row[9] {
        // BigDecimal::to_string preserves every input digit.
        assert!(n.contains("123456789012345.6789"));
    } else {
        panic!("expected numeric");
    }
}

#[tokio::test]
async fn execute_query_caps_to_default_limit() {
    let live = boot().await;
    let driver = PostgresDriver::new();
    let conn = driver
        .connect(&live.config, Some(&live.password))
        .await
        .expect("connect");

    let big = driver
        .execute_query(conn.as_ref(), &Query::new("SELECT generate_series(1, 700)"))
        .await
        .expect("query");
    assert_eq!(big.rows.len(), 500);
    assert!(big.truncated);

    let small = driver
        .execute_query(
            conn.as_ref(),
            &Query::new("SELECT generate_series(1, 10)").with_max_rows(50),
        )
        .await
        .expect("query");
    assert_eq!(small.rows.len(), 10);
    assert!(!small.truncated);
}
