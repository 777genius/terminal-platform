use thiserror::Error;

use terminal_domain::DegradedModeReason;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendErrorKind {
    Unsupported,
    NotFound,
    InvalidInput,
    Transport,
    Internal,
}

#[derive(Debug, Error)]
#[error("{kind:?}: {message}")]
pub struct BackendError {
    pub kind: BackendErrorKind,
    pub message: String,
    pub degraded_reason: Option<DegradedModeReason>,
}

impl BackendError {
    #[must_use]
    pub fn new(kind: BackendErrorKind, message: impl Into<String>) -> Self {
        Self { kind, message: message.into(), degraded_reason: None }
    }

    #[must_use]
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(BackendErrorKind::NotFound, message)
    }

    #[must_use]
    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::new(BackendErrorKind::InvalidInput, message)
    }

    #[must_use]
    pub fn unsupported(message: impl Into<String>, degraded_reason: DegradedModeReason) -> Self {
        Self {
            kind: BackendErrorKind::Unsupported,
            message: message.into(),
            degraded_reason: Some(degraded_reason),
        }
    }
}
