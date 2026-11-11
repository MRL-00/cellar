use super::*;
use cellar_core::driver::{Engine, SshHostConfig, SshTunnelConfig, SslMode};

fn name(kind: Kind) -> &'static str {
    match kind {
        Kind::Password => "password",
        Kind::SshPassword => "ssh-password",
        Kind::SshKeyPassphrase => "ssh-passphrase",
        Kind::JumpPassword => "jump-password",
        Kind::JumpKeyPassphrase => "jump-passphrase",
    }
}

fn host(auth_method: SshAuthMethod) -> SshHostConfig {
    SshHostConfig {
        host: "bastion".into(),
        port: 22,
        user: "deploy".into(),
        auth_method,
        private_key_path: None,
    }
}

fn config(server: Option<SshAuthMethod>, jump: Option<SshAuthMethod>) -> ConnectionConfig {
    ConnectionConfig {
        id: "conn".into(),
        name: "conn".into(),
        engine: Engine::Postgres,
        host: "db".into(),
        port: 5432,
        database: "app".into(),
        user: "app".into(),
        ssl_mode: SslMode::Prefer,
        ssh: server.map(|server| SshTunnelConfig {
            server: host(server),
            jump_host: jump.map(host),
        }),
        env_tag: None,
        application_name: None,
        color: None,
    }
}

fn all(value: &str) -> ConnectionSecrets {
    ConnectionSecrets {
        password: Some(value.into()),
        ssh_password: Some(value.into()),
        ssh_key_passphrase: Some(value.into()),
        jump_password: Some(value.into()),
        jump_key_passphrase: Some(value.into()),
    }
}

type Projected<'a> = (Vec<(&'static str, Option<&'a str>)>, Vec<&'static str>);

fn project<'a>(plan: SecretPlan<'a>) -> Projected<'a> {
    let changes = plan
        .changes
        .into_iter()
        .map(|(kind, change)| match change {
            Change::Store(value) => (name(kind), Some(value)),
            Change::Delete => (name(kind), None),
        })
        .collect();
    (changes, plan.cleanup.into_iter().map(name).collect())
}

#[test]
fn ssh_disabled_ignores_supplied_values_and_cleans_every_ssh_kind() {
    let secrets = all("x");
    let (changes, cleanup) = project(plan_secret_changes(&config(None, None), &secrets));
    assert_eq!(changes, [("password", Some("x"))]);
    assert_eq!(
        cleanup,
        [
            "ssh-password",
            "ssh-passphrase",
            "jump-password",
            "jump-passphrase"
        ]
    );
}

#[test]
fn only_kinds_for_configured_auth_methods_are_required() {
    let secrets = all("x");
    let key_and_password = config(
        Some(SshAuthMethod::PrivateKey),
        Some(SshAuthMethod::Password),
    );
    let (changes, cleanup) = project(plan_secret_changes(&key_and_password, &secrets));
    assert_eq!(
        changes,
        [
            ("password", Some("x")),
            ("ssh-passphrase", Some("x")),
            ("jump-password", Some("x")),
        ]
    );
    assert_eq!(cleanup, ["ssh-password", "jump-passphrase"]);

    let agent = config(Some(SshAuthMethod::Agent), None);
    let (changes, cleanup) = project(plan_secret_changes(&agent, &secrets));
    assert_eq!(changes, [("password", Some("x"))]);
    assert_eq!(cleanup.len(), 4);
}

#[test]
fn empty_clears_ssh_kinds_but_stores_empty_database_password() {
    let secrets = all("");
    let config = config(
        Some(SshAuthMethod::Password),
        Some(SshAuthMethod::PrivateKey),
    );
    let (changes, cleanup) = project(plan_secret_changes(&config, &secrets));
    assert_eq!(
        changes,
        [
            ("password", Some("")),
            ("ssh-password", None),
            ("jump-passphrase", None),
        ]
    );
    assert_eq!(cleanup, ["ssh-passphrase", "jump-password"]);
}

#[test]
fn absent_values_keep_required_entries() {
    let config = config(
        Some(SshAuthMethod::PrivateKey),
        Some(SshAuthMethod::PrivateKey),
    );
    let secrets = ConnectionSecrets::default();
    let (changes, cleanup) = project(plan_secret_changes(&config, &secrets));
    assert!(changes.is_empty());
    assert_eq!(cleanup, ["ssh-password", "jump-password"]);
}

#[test]
fn resolve_treats_empty_and_unused_ssh_values_as_absent() {
    // Every required kind is supplied, so resolve never reads the keychain
    // except for the database password, which is also supplied here.
    let config = config(
        Some(SshAuthMethod::PrivateKey),
        Some(SshAuthMethod::Password),
    );
    let mut secrets = all("");
    secrets.password = Some("db".into());
    secrets.jump_password = Some("jump".into());
    secrets.ssh_password = Some("unused".into());
    let resolved = resolve(&config, secrets).unwrap();
    assert_eq!(resolved.password.as_deref(), Some("db"));
    assert_eq!(resolved.ssh_key_passphrase, None);
    assert_eq!(resolved.jump_password.as_deref(), Some("jump"));
    assert_eq!(resolved.ssh_password, None);
    assert_eq!(resolved.jump_key_passphrase, None);
}
