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
async fn execute_query_decodes_user_defined_enums() {
    // Regression: enum columns hit the decode fallback. sqlx's String decode
    // rejects the custom type OID, so the driver now reads the raw value bytes
    // (which carry the enum label) as UTF-8. Before the fix this surfaced as
    // `UnsupportedType` and failed the whole table/view load.
    let live = boot().await;
    let driver = PostgresDriver::new();
    let conn = driver
        .connect(&live.config, Some(&live.password))
        .await
        .expect("connect");

    driver
        .execute_query(
            conn.as_ref(),
            &Query::new("CREATE TYPE mood AS ENUM ('sad', 'ok', 'happy')"),
        )
        .await
        .expect("create enum type");

    let result = driver
        .execute_query(
            conn.as_ref(),
            &Query::new("SELECT 'happy'::mood AS m, NULL::mood AS n"),
        )
        .await
        .expect("query enum");

    assert_eq!(result.rows.len(), 1);
    if let CellValue::Text(m) = &result.rows[0][0] {
        assert_eq!(m, "happy");
    } else {
        panic!("expected enum decoded as text, got {:?}", result.rows[0][0]);
    }
    assert!(matches!(result.rows[0][1], CellValue::Null));
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

#[tokio::test]
async fn cancel_query_stops_a_running_statement() {
    let live = boot().await;
    let driver = PostgresDriver::new();
    let conn = driver
        .connect(&live.config, Some(&live.password))
        .await
        .expect("connect");

    // Unknown ids are a no-op, not an error.
    assert!(!driver
        .cancel_query(conn.as_ref(), "never-started")
        .await
        .expect("cancel unknown id"));

    let conn = std::sync::Arc::new(conn);
    let runner = {
        let conn = std::sync::Arc::clone(&conn);
        tokio::spawn(async move {
            PostgresDriver::new()
                .execute_query(
                    conn.as_ref().as_ref(),
                    &Query::new("SELECT pg_sleep(30)").with_query_id("cancel-me"),
                )
                .await
        })
    };

    // Signal repeatedly until the statement dies: the first attempts may land
    // before the statement registers or while the backend is still idle, and
    // pg_cancel_backend only interrupts a statement that is already running.
    for _ in 0..300 {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let _ = driver
            .cancel_query(conn.as_ref().as_ref(), "cancel-me")
            .await;
        if runner.is_finished() {
            break;
        }
    }
    assert!(runner.is_finished(), "query was not cancelled within 30s");

    let err = runner
        .await
        .expect("join runner task")
        .expect_err("cancelled query should report an error");
    let msg = err.to_string().to_lowercase();
    assert!(msg.contains("cancel"), "unexpected error: {msg}");

    // The registration is cleaned up after the run settles.
    assert!(!driver
        .cancel_query(conn.as_ref().as_ref(), "cancel-me")
        .await
        .expect("cancel after completion"));
}

#[tokio::test]
async fn execute_query_reports_rows_affected_for_dml_only() {
    let live = boot().await;
    let driver = PostgresDriver::new();
    let conn = driver
        .connect(&live.config, Some(&live.password))
        .await
        .expect("connect");

    let insert = driver
        .execute_query(
            conn.as_ref(),
            &Query::new(
                "INSERT INTO customers (email) VALUES ('a@example.com'), ('b@example.com')",
            ),
        )
        .await
        .expect("insert");
    assert_eq!(insert.rows_affected, Some(2));
    assert!(insert.rows.is_empty());

    let update = driver
        .execute_query(
            conn.as_ref(),
            &Query::new(
                "UPDATE customers SET email = email || '.x' WHERE email LIKE '%@example.com'",
            ),
        )
        .await
        .expect("update");
    assert_eq!(update.rows_affected, Some(2));

    // Row-returning statements must not surface the command tag's count:
    // the UI would mislabel a SELECT as "N rows affected".
    let select = driver
        .execute_query(conn.as_ref(), &Query::new("SELECT * FROM customers"))
        .await
        .expect("select");
    assert_eq!(select.rows_affected, None);

    let returning = driver
        .execute_query(
            conn.as_ref(),
            &Query::new("DELETE FROM customers WHERE email LIKE '%@example.com.x' RETURNING id"),
        )
        .await
        .expect("delete returning");
    assert_eq!(returning.rows_affected, None);
    assert_eq!(returning.rows.len(), 2);
}
