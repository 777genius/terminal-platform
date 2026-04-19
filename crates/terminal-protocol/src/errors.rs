use std::io;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Error)]
#[error("{code}: {message}")]
pub struct ProtocolError {
    pub code: String,
    pub message: String,
}

impl ProtocolError {
    #[must_use]
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self { code: code.into(), message: message.into() }
    }

    #[must_use]
    pub fn serialize(error: serde_json::Error) -> Self {
        Self::new("serialize_failed", error.to_string())
    }

    #[must_use]
    pub fn deserialize(error: serde_json::Error) -> Self {
        Self::new("deserialize_failed", error.to_string())
    }

    #[must_use]
    pub fn io(code: impl Into<String>, error: &io::Error) -> Self {
        Self::new(code, error.to_string())
    }

    #[must_use]
    pub fn unexpected_payload(expected: &str, actual: impl std::fmt::Debug) -> Self {
        Self::new("unexpected_payload", format!("expected {expected}, got {actual:?}"))
    }
}
