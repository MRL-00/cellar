use cellar_core::driver::{ConnectionConfig, Engine, SslMode};

#[test]
fn database_options_preserve_literal_identifiers_and_ipv6_host() {
    let mut config = ConnectionConfig {
        id: "test".into(),
        name: "test".into(),
        engine: Engine::Postgres,
        host: "localhost".into(),
        port: 5432,
        database: "postgres".into(),
        user: "postgres".into(),
        ssl_mode: SslMode::Disable,
        env_tag: None,
        application_name: None,
        color: None,
        ssh: None,
    };
    config.host = "::1".into();
    config.user = "user@:/?#%20".into();
    config.database = "db/?#%2F".into();
    config.ssl_mode = SslMode::VerifyFull;
    let options = crate::connect::connect_options(&config, Some("password"), None).unwrap();
    assert_eq!(options.get_host(), "::1");
    assert_eq!(options.get_username(), config.user);
    assert_eq!(options.get_database(), Some(config.database.as_str()));
    assert!(matches!(
        options.get_ssl_mode(),
        sqlx::postgres::PgSslMode::VerifyFull
    ));
}
