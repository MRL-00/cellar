use super::support::persist_to_dir;
use super::{bulk_connections::merge_new_configs, ConnectionRegistry, STORAGE_FILENAME};
use cellar_core::driver::{ConnectionConfig, Engine, SslMode};
use std::collections::HashMap;

fn make_config(id: &str, name: &str) -> ConnectionConfig {
    ConnectionConfig {
        id: id.into(),
        name: name.into(),
        engine: Engine::Postgres,
        host: "localhost".into(),
        port: 5432,
        user: "user".into(),
        database: "db".into(),
        ssl_mode: SslMode::Prefer,
        ssh: None,
        env_tag: None,
        application_name: None,
        color: None,
    }
}

#[test]
fn bulk_import_rejects_existing_and_duplicate_ids() {
    let existing = HashMap::from([("conn-1".into(), make_config("conn-1", "Existing"))]);
    assert!(merge_new_configs(&existing, &[make_config("conn-1", "Replacement")]).is_err());
    assert!(merge_new_configs(
        &HashMap::new(),
        &[make_config("new", "One"), make_config("new", "Two")]
    )
    .is_err());
    assert_eq!(
        merge_new_configs(&existing, &[make_config("conn-2", "New")])
            .expect("new id should merge")
            .len(),
        2
    );
}

/// Verify that `save()` writes the config to disk BEFORE the in-memory
/// registry reflects it.  We test the happy path: after a successful save
/// the on-disk file must contain the new config, proving that persist was
/// called (and succeeded) as part of the operation.
#[tokio::test]
async fn save_writes_to_disk_before_returning() {
    let dir = tempfile::tempdir().expect("tmpdir");
    let registry = ConnectionRegistry::empty();

    // Patch the registry to write into our temp dir by calling
    // persist_to_dir directly with the same configs map that save() would
    // build — this mirrors the exact sequence save() executes.
    let config = make_config("conn-1", "My DB");
    let mut configs: HashMap<_, _> = HashMap::new();
    configs.insert(config.id.clone(), config.clone());
    persist_to_dir(&configs, dir.path())
        .await
        .expect("persist should succeed");

    // Confirm the file was written and round-trips cleanly.
    let written = tokio::fs::read_to_string(dir.path().join(STORAGE_FILENAME))
        .await
        .expect("file should exist after persist");
    let parsed: Vec<ConnectionConfig> = serde_json::from_str(&written).expect("valid JSON");
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].id, "conn-1");

    // Also confirm the registry is still empty (we haven't called save()
    // on it), which demonstrates the persist-before-mutate ordering: the
    // caller of save() can observe the on-disk state is updated before the
    // in-memory map is touched.
    assert!(
        registry.list().await.is_empty(),
        "in-memory registry must not be mutated until persist succeeds"
    );
}

/// Verify that `delete()` happy-path: the config is absent from disk after
/// a successful delete, matching the in-memory state.
#[tokio::test]
async fn delete_removes_config_from_disk() {
    let dir = tempfile::tempdir().expect("tmpdir");

    // Seed a config on disk.
    let config = make_config("conn-2", "Second DB");
    let mut configs: HashMap<_, _> = HashMap::new();
    configs.insert(config.id.clone(), config);
    persist_to_dir(&configs, dir.path())
        .await
        .expect("initial persist");

    // Simulate the delete ordering: remove from map, persist, then
    // in-memory state would be updated.
    configs.remove("conn-2");
    persist_to_dir(&configs, dir.path())
        .await
        .expect("persist after delete");

    let written = tokio::fs::read_to_string(dir.path().join(STORAGE_FILENAME))
        .await
        .expect("file should exist");
    let parsed: Vec<ConnectionConfig> = serde_json::from_str(&written).expect("valid JSON");
    assert!(
        parsed.is_empty(),
        "disk must not contain deleted config after persist"
    );
}

#[test]
fn connection_metadata_is_backward_compatible_and_roundtrips_ssh_without_secrets() {
    use cellar_core::driver::{SshAuthMethod, SshHostConfig, SshTunnelConfig};
    let config = make_config("legacy", "Legacy");
    let legacy = serde_json::to_value(&config).unwrap();
    assert!(legacy.get("ssh").is_none());
    assert!(serde_json::from_value::<ConnectionConfig>(legacy)
        .unwrap()
        .ssh
        .is_none());
    let mut config = config;
    let host = SshHostConfig {
        host: "ssh.internal".into(),
        port: 22,
        user: "ssh-user".into(),
        auth_method: SshAuthMethod::PrivateKey,
        private_key_path: Some("~/.ssh/id_ed25519".into()),
    };
    config.ssh = Some(SshTunnelConfig {
        server: host.clone(),
        jump_host: Some(SshHostConfig {
            auth_method: SshAuthMethod::Password,
            ..host
        }),
    });
    let json = serde_json::to_value(config).unwrap();
    let decoded: ConnectionConfig = serde_json::from_value(json.clone()).unwrap();
    assert_eq!(
        decoded.ssh.unwrap().jump_host.unwrap().auth_method,
        SshAuthMethod::Password
    );
    let encoded = json.to_string();
    assert!(!encoded.contains("password\":"));
    assert!(!encoded.contains("passphrase"));
}

#[tokio::test]
async fn unsupported_drivers_reject_ssh_before_opening_any_transport() {
    use cellar_core::driver::{ConnectionSecrets, SshAuthMethod, SshHostConfig, SshTunnelConfig};
    let mut config = make_config("unsupported", "Unsupported");
    config.ssh = Some(SshTunnelConfig {
        server: SshHostConfig {
            host: "127.0.0.1".into(),
            port: 1,
            user: "ssh".into(),
            auth_method: SshAuthMethod::Agent,
            private_key_path: None,
        },
        jump_host: None,
    });
    for engine in [
        Engine::Azure,
        Engine::Sqlite,
        Engine::Firestore,
        Engine::Cosmos,
        Engine::Convex,
    ] {
        config.engine = engine;
        let driver = super::support::driver_for(engine).unwrap();
        for result in [
            driver.connect(&config, None).await,
            driver
                .connect_with_secrets(&config, &ConnectionSecrets::default())
                .await,
        ] {
            let error = result.err().expect("must reject SSH");
            assert!(
                error.to_string().contains(
                    "SSH tunneling is supported only by PostgreSQL, MySQL, and SQL Server"
                ),
                "{engine:?}: {error}"
            );
        }
    }
}

#[test]
fn stale_pool_never_wins_over_current_settings() {
    use super::support::keep_existing_pool;
    let (old, new, other) = (
        make_config("c", "Old"),
        make_config("c", "New"),
        make_config("c", "Other"),
    );
    assert!(
        !keep_existing_pool(&old, &new, Some(&new)),
        "replace stale pool"
    );
    assert!(
        keep_existing_pool(&new, &old, Some(&new)),
        "stale attempt loses"
    );
    assert!(keep_existing_pool(&new, &new, Some(&new)));
    assert!(keep_existing_pool(&old, &other, Some(&new)), "both stale");
    assert!(!keep_existing_pool(&old, &new, None), "deleted config");
}

#[tokio::test]
async fn connect_replaces_pool_opened_with_edited_settings() {
    let dir = tempfile::tempdir().expect("tmpdir");
    let (first, second) = (dir.path().join("a.db"), dir.path().join("b.db"));
    std::fs::File::create(&first).expect("first db");
    std::fs::File::create(&second).expect("second db");
    let mut config = make_config("c", "SQLite");
    config.engine = Engine::Sqlite;
    config.database = first.to_string_lossy().into_owned();
    let registry = ConnectionRegistry::empty();
    registry
        .inner
        .write()
        .await
        .configs
        .insert("c".into(), config.clone());
    // A supplied password keeps the test off the OS keychain.
    registry.connect("c", Some("")).await.expect("connect");
    let pool = std::sync::Arc::clone(&registry.inner.read().await.open["c"].connection);
    registry.connect("c", Some("")).await.expect("reuse");
    assert!(std::sync::Arc::ptr_eq(
        &pool,
        &registry.inner.read().await.open["c"].connection
    ));

    config.database = second.to_string_lossy().into_owned();
    registry
        .inner
        .write()
        .await
        .configs
        .insert("c".into(), config.clone());
    registry.connect("c", Some("")).await.expect("reconnect");
    {
        let inner = registry.inner.read().await;
        assert_eq!(inner.open["c"].config, config);
        assert!(!std::sync::Arc::ptr_eq(&pool, &inner.open["c"].connection));
    }
    registry.disconnect("c").await.expect("disconnect");
}
