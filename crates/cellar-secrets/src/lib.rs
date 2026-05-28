//! OS-keychain-backed credential storage for Cellar.
//!
//! Per SPEC §5.3 and §7 the OS keychain is the canonical store. An
//! encrypted-file fallback (master-password-derived key) is required for
//! environments without a keychain — that's tracked in
//! [`docs/architecture/adr/0001-secret-fallback.md`] and intentionally not
//! built yet.
//!
//! Nothing in this module logs or formats the password.

use cellar_core::error::CellarError;

const SERVICE: &str = "com.cellar.desktop";

#[derive(Debug, thiserror::Error)]
pub enum SecretError {
    #[error("no secret found for {0}")]
    NotFound(String),

    #[error("keyring error: {0}")]
    Keyring(String),
}

impl From<keyring::Error> for SecretError {
    fn from(value: keyring::Error) -> Self {
        match value {
            keyring::Error::NoEntry => Self::NotFound(String::new()),
            // The keyring crate's error `Display` is safe to surface, but we
            // still avoid embedding any caller-supplied key material here.
            other => Self::Keyring(other.to_string()),
        }
    }
}

impl From<SecretError> for CellarError {
    fn from(value: SecretError) -> Self {
        match value {
            SecretError::NotFound(name) => {
                CellarError::Authentication(format!("no stored password for {name}"))
            }
            SecretError::Keyring(msg) => CellarError::Authentication(msg),
        }
    }
}

/// Persist `password` under `name` in the OS keychain. Overwrites any
/// existing entry. The password value never leaves this scope, is not logged,
/// and is not held by the returned future.
pub fn store(name: &str, password: &str) -> Result<(), SecretError> {
    let entry = keyring::Entry::new(SERVICE, name)?;
    entry.set_password(password)?;
    Ok(())
}

/// Load the password stored under `name`. Returns `Ok(None)` when no entry
/// exists, which is distinct from a real keyring failure.
pub fn load(name: &str) -> Result<Option<String>, SecretError> {
    let entry = keyring::Entry::new(SERVICE, name)?;
    match entry.get_password() {
        Ok(p) => Ok(Some(p)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(other) => Err(other.into()),
    }
}

/// Remove the entry stored under `name`. A no-op if no entry exists.
pub fn delete(name: &str) -> Result<(), SecretError> {
    let entry = keyring::Entry::new(SERVICE, name)?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(other) => Err(other.into()),
    }
}
