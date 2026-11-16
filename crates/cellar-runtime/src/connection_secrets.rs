use cellar_core::driver::{ConnectionConfig, ConnectionSecrets, DriverInfo, SshAuthMethod};
use cellar_core::error::{CellarError, CellarResult};
use cellar_secrets::{
    delete_connection, load_connection, store_connection, ConnectionSecretKind as Kind,
};

use crate::{
    support::{driver_for, persist},
    ConnectionRegistry,
};

impl ConnectionRegistry {
    /// Database password: `None` keeps the stored value, `Some("")` stores an
    /// empty password. SSH kinds the config uses: `None` keeps, `Some("")`
    /// clears, otherwise stores. SSH kinds the config does not use are deleted
    /// and supplied values for them are ignored.
    pub async fn save_with_secrets(
        &self,
        config: ConnectionConfig,
        secrets: &ConnectionSecrets,
    ) -> CellarResult<ConnectionConfig> {
        if config.id.is_empty() {
            return Err(CellarError::invalid_config("connection id is empty"));
        }
        let mut inner = self.inner.write().await;
        let plan = plan_secret_changes(&config, secrets);
        // Snapshot entries being changed. Never change an unreadable keychain
        // entry, since its old value could not be restored on failure.
        let mut changes = Vec::new();
        for (kind, change) in plan.changes {
            changes.push((kind, change, load_connection(&config.id, kind)?));
        }
        let mut updated = inner.configs.clone();
        updated.insert(config.id.clone(), config.clone());
        persist(&updated).await?;
        for (index, (kind, change, _)) in changes.iter().enumerate() {
            let result = match change {
                Change::Store(value) => store_connection(&config.id, *kind, value),
                Change::Delete => delete_connection(&config.id, *kind),
            };
            if let Err(error) = result {
                // Include the failing entry: an OS backend may have written
                // before reporting an error. Attempt every rollback operation.
                let mut rollback_failed = false;
                for (kind, _, previous) in changes[..=index].iter().rev() {
                    let result = match previous {
                        Some(value) => store_connection(&config.id, *kind, value),
                        None => delete_connection(&config.id, *kind),
                    };
                    rollback_failed |= result.is_err();
                }
                rollback_failed |= persist(&inner.configs).await.is_err();
                if rollback_failed {
                    return Err(CellarError::Internal("credential save failed and rollback was incomplete; review saved connection settings".into()));
                }
                return Err(error.into());
            }
        }
        inner.configs = updated;
        // Unused SSH entries are removed only after the save is committed, so
        // they never need rollback and an unreadable entry cannot block saving.
        // resolve() never reads unused kinds; a failed delete only leaves an
        // entry that a later save clears or the user overwrites.
        for kind in plan.cleanup {
            let _ = delete_connection(&config.id, kind);
        }
        Ok(config)
    }

    pub async fn test_with_secrets(
        &self,
        config: &ConnectionConfig,
        secrets: ConnectionSecrets,
    ) -> CellarResult<DriverInfo> {
        let secrets = resolve(config, secrets)?;
        let driver = driver_for(config.engine)?;
        let connection = driver.connect_with_secrets(config, &secrets).await?;
        let info = connection.info().clone();
        connection.close().await?;
        Ok(info)
    }
}

/// Keychain operation; deliberately not `Debug` because it may hold a secret.
enum Change<'a> {
    Store(&'a str),
    Delete,
}

struct SecretPlan<'a> {
    /// Transactional changes, snapshotted and rolled back on failure.
    changes: Vec<(Kind, Change<'a>)>,
    /// SSH kinds the config no longer uses; deleted best-effort after commit.
    cleanup: Vec<Kind>,
}

fn ssh_kinds(secrets: &ConnectionSecrets) -> [(Kind, &Option<String>); 4] {
    [
        (Kind::SshPassword, &secrets.ssh_password),
        (Kind::SshKeyPassphrase, &secrets.ssh_key_passphrase),
        (Kind::JumpPassword, &secrets.jump_password),
        (Kind::JumpKeyPassphrase, &secrets.jump_key_passphrase),
    ]
}

/// Whether `config` authenticates with the SSH secret `kind`. The database
/// password is not an SSH kind and always returns false.
fn ssh_kind_required(config: &ConnectionConfig, kind: Kind) -> bool {
    let Some(ssh) = &config.ssh else {
        return false;
    };
    let jump = ssh.jump_host.as_ref().map(|jump| jump.auth_method);
    match kind {
        Kind::Password => false,
        Kind::SshPassword => ssh.server.auth_method == SshAuthMethod::Password,
        Kind::SshKeyPassphrase => ssh.server.auth_method == SshAuthMethod::PrivateKey,
        Kind::JumpPassword => jump == Some(SshAuthMethod::Password),
        Kind::JumpKeyPassphrase => jump == Some(SshAuthMethod::PrivateKey),
    }
}

fn plan_secret_changes<'a>(
    config: &ConnectionConfig,
    secrets: &'a ConnectionSecrets,
) -> SecretPlan<'a> {
    let mut plan = SecretPlan {
        changes: Vec::new(),
        cleanup: Vec::new(),
    };
    if let Some(password) = &secrets.password {
        plan.changes.push((Kind::Password, Change::Store(password)));
    }
    for (kind, value) in ssh_kinds(secrets) {
        if !ssh_kind_required(config, kind) {
            plan.cleanup.push(kind);
            continue;
        }
        match value.as_deref() {
            None => {}
            Some("") => plan.changes.push((kind, Change::Delete)),
            Some(value) => plan.changes.push((kind, Change::Store(value))),
        }
    }
    plan
}

pub(crate) fn resolve(
    config: &ConnectionConfig,
    mut secrets: ConnectionSecrets,
) -> CellarResult<ConnectionSecrets> {
    // Preserve the legacy passwordless fallback when the OS keychain is
    // unavailable. SSH credentials remain strict and never silently fall back.
    if secrets.password.is_none() {
        secrets.password = load_connection(&config.id, Kind::Password).ok().flatten();
    }
    for (kind, value) in [
        (Kind::SshPassword, &mut secrets.ssh_password),
        (Kind::SshKeyPassphrase, &mut secrets.ssh_key_passphrase),
        (Kind::JumpPassword, &mut secrets.jump_password),
        (Kind::JumpKeyPassphrase, &mut secrets.jump_key_passphrase),
    ] {
        if !ssh_kind_required(config, kind) {
            *value = None;
            continue;
        }
        if value.is_none() {
            *value = load_connection(&config.id, kind)?;
        }
        // Empty means absent: a supplied passphrase makes russh decrypt the
        // key, which fails for unencrypted PKCS#8 keys.
        if value.as_deref() == Some("") {
            *value = None;
        }
    }
    Ok(secrets)
}

pub(crate) fn delete_all(id: &str) {
    for kind in [
        Kind::Password,
        Kind::SshPassword,
        Kind::SshKeyPassphrase,
        Kind::JumpPassword,
        Kind::JumpKeyPassphrase,
    ] {
        let _ = delete_connection(id, kind);
    }
}

#[cfg(test)]
#[path = "connection_secrets_tests.rs"]
mod tests;
