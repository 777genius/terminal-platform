use crate::dto::{prelude::*, *};

impl From<&SessionHealthPhase> for NodeSessionHealthPhase {
    fn from(value: &SessionHealthPhase) -> Self {
        match value {
            SessionHealthPhase::Ready => Self::Ready,
            SessionHealthPhase::Degraded => Self::Degraded,
            SessionHealthPhase::Stale => Self::Stale,
            SessionHealthPhase::Terminated => Self::Terminated,
        }
    }
}

impl From<&SessionHealthReason> for NodeSessionHealthReason {
    fn from(value: &SessionHealthReason) -> Self {
        match value {
            SessionHealthReason::BackendDegraded => Self::BackendDegraded,
            SessionHealthReason::SubscriptionSourceClosed => Self::SubscriptionSourceClosed,
            SessionHealthReason::SessionNotFound => Self::SessionNotFound,
            SessionHealthReason::BackendTransportLost => Self::BackendTransportLost,
            SessionHealthReason::BackendInternalFault => Self::BackendInternalFault,
        }
    }
}

impl From<&SessionHealthSnapshot> for NodeSessionHealthSnapshot {
    fn from(value: &SessionHealthSnapshot) -> Self {
        Self {
            session_id: value.session_id.0.to_string(),
            phase: (&value.phase).into(),
            can_attach: value.can_attach,
            invalidated: value.invalidated,
            reason: value.reason.as_ref().map(Into::into),
            detail: value.detail.clone(),
        }
    }
}
