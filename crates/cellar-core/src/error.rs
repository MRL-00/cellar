use serde::{Deserialize, Serialize};
use specta::Type;
use thiserror::Error;

/// Typed error surface for every public API in `cellar-core` and driver crates.
///
/// Crossing the IPC boundary requires serializable values, so the error is a
/// closed enum rather than a string. Drivers map their native errors into one
/// of these variants and stash the original message in `detail`.
#[derive(Debug, Error, Clone, Serialize, Deserialize, Type)]
#[serde(tag = "kind", content = "detail")]
pub enum CellarError {
    #[error("connection failed: {0}")]
    Connection(String),

    #[error("authentication failed: {0}")]
    Authentication(String),

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
