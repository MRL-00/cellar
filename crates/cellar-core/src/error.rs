use serde::{Deserialize, Serialize};
use specta::Type;
use thiserror::Error;

/// Public host key presented by an untrusted SSH peer. Contains no private material.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct SshHostKey {
    pub host: String,
    pub port: u16,
    pub algorithm: String,
    pub fingerprint: String,
    pub public_key: String,
}

/// Typed error surface for every public API in `cellar-core` and driver crates.
///
/// Runtime and persisted error values use a closed enum rather than strings.
/// Drivers map native errors into these variants and retain their detail.
#[derive(Debug, Error, Clone, Serialize, Deserialize, Type)]
#[serde(tag = "kind", content = "detail")]
pub enum CellarError {
    #[error("connection failed: {0}")]
    Connection(String),

    #[error("authentication failed: {0}")]
    Authentication(String),

    #[error("SSH host {host}:{} is unknown ({}, {}); verify its fingerprint before trusting", .0.port, .0.algorithm, .0.fingerprint, host = .0.host)]
    SshHostKeyUnknown(SshHostKey),

    #[error("tls handshake failed: {0}")]
    Tls(String),

    #[error("query failed: {0}")]
    Query(String),

    #[error("introspection failed: {0}")]
    Introspection(String),

    #[error("unsupported type for engine: {0}")]
    UnsupportedType(String),

    #[error("decode error: {0}")]
    Decode(String),

    #[error("timeout: {0}")]
    Timeout(String),

    #[error("not connected: {0}")]
    NotConnected(String),

    #[error("invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("io error: {0}")]
    Io(String),

    #[error("internal error: {0}")]
    Internal(String),
}

impl CellarError {
    pub fn invalid_config(msg: impl Into<String>) -> Self {
        Self::InvalidConfig(msg.into())
    }

    pub fn connection(msg: impl Into<String>) -> Self {
        Self::Connection(msg.into())
    }

    pub fn query(msg: impl Into<String>) -> Self {
        Self::Query(msg.into())
    }

    pub fn decode(msg: impl Into<String>) -> Self {
        Self::Decode(msg.into())
    }

    pub fn introspection(msg: impl Into<String>) -> Self {
        Self::Introspection(msg.into())
    }
}

pub type CellarResult<T> = Result<T, CellarError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssh_unknown_challenge_round_trips_with_public_details() {
        let error = CellarError::SshHostKeyUnknown(SshHostKey {
            host: "db.example".into(),
            port: 2222,
            algorithm: "ssh-ed25519".into(),
            fingerprint: "SHA256:example".into(),
            public_key: "ssh-ed25519 example".into(),
        });
        let json = serde_json::to_string(&error).unwrap();
        let decoded: CellarError = serde_json::from_str(&json).unwrap();
        let CellarError::SshHostKeyUnknown(key) = &decoded else {
            panic!("challenge variant lost");
        };
        assert_eq!(key.host, "db.example");
        assert_eq!(key.port, 2222);
        assert_eq!(key.public_key, "ssh-ed25519 example");
        assert!(decoded.to_string().contains("db.example:2222"));
        assert!(decoded.to_string().contains("SHA256:example"));
    }
}

impl From<std::io::Error> for CellarError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value.to_string())
    }
}

impl From<serde_json::Error> for CellarError {
    fn from(value: serde_json::Error) -> Self {
        Self::Decode(value.to_string())
    }
}
