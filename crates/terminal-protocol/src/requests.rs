use serde::{Deserialize, Serialize};

use terminal_backend_api::{CreateSessionSpec, SubscriptionSpec};
use terminal_domain::{BackendKind, SessionId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateSessionRequest {
    pub backend: BackendKind,
    pub spec: CreateSessionSpec,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenSubscriptionRequest {
    pub session_id: SessionId,
    pub spec: SubscriptionSpec,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RequestPayload {
    Handshake,
    CreateSession(CreateSessionRequest),
    ListSessions,
    OpenSubscription(OpenSubscriptionRequest),
}
