use serde::{Deserialize, Serialize};
use terminal_domain::SessionId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionHealthPhase {
    Ready,
    Degraded,
    Stale,
    Terminated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionHealthReason {
    BackendDegraded,
    SubscriptionSourceClosed,
    SessionNotFound,
    BackendTransportLost,
    BackendInternalFault,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionHealthSnapshot {
    pub session_id: SessionId,
    pub phase: SessionHealthPhase,
    pub can_attach: bool,
    pub invalidated: bool,
    pub reason: Option<SessionHealthReason>,
    pub detail: Option<String>,
}

impl SessionHealthSnapshot {
    #[must_use]
    pub fn ready(session_id: SessionId) -> Self {
        Self {
            session_id,
            phase: SessionHealthPhase::Ready,
            can_attach: true,
            invalidated: false,
            reason: None,
            detail: None,
        }
    }

    #[must_use]
    pub fn degraded(
        session_id: SessionId,
        reason: SessionHealthReason,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            session_id,
            phase: SessionHealthPhase::Degraded,
            can_attach: true,
            invalidated: false,
            reason: Some(reason),
            detail: Some(detail.into()),
        }
    }

    #[must_use]
    pub fn stale(
        session_id: SessionId,
        reason: SessionHealthReason,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            session_id,
            phase: SessionHealthPhase::Stale,
            can_attach: false,
            invalidated: true,
            reason: Some(reason),
            detail: Some(detail.into()),
        }
    }

    #[must_use]
    pub fn terminated(
        session_id: SessionId,
        reason: SessionHealthReason,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            session_id,
            phase: SessionHealthPhase::Terminated,
            can_attach: false,
            invalidated: true,
            reason: Some(reason),
            detail: Some(detail.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use terminal_domain::SessionId;

    use super::{SessionHealthPhase, SessionHealthReason, SessionHealthSnapshot};

    #[test]
    fn stale_and_terminated_snapshots_are_invalidated() {
        let session_id = SessionId::new();
        let stale = SessionHealthSnapshot::stale(
            session_id,
            SessionHealthReason::SubscriptionSourceClosed,
            "subscription source closed",
        );
        let terminated = SessionHealthSnapshot::terminated(
            session_id,
            SessionHealthReason::SessionNotFound,
            "session disappeared",
        );

        assert_eq!(stale.phase, SessionHealthPhase::Stale);
        assert!(stale.invalidated);
        assert!(!stale.can_attach);
        assert_eq!(terminated.phase, SessionHealthPhase::Terminated);
        assert!(terminated.invalidated);
        assert!(!terminated.can_attach);
    }
}
