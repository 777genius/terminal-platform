use serde::{Deserialize, Serialize};

use terminal_backend_api::{CreateSessionSpec, MuxCommand, SubscriptionSpec};
use terminal_domain::{BackendKind, PaneId, SessionId};

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
pub struct GetTopologySnapshotRequest {
    pub session_id: SessionId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetScreenSnapshotRequest {
    pub session_id: SessionId,
    pub pane_id: PaneId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetScreenDeltaRequest {
    pub session_id: SessionId,
    pub pane_id: PaneId,
    pub from_sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispatchMuxCommandRequest {
    pub session_id: SessionId,
    pub command: MuxCommand,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RequestPayload {
    Handshake,
    CreateSession(CreateSessionRequest),
    ListSessions,
    GetTopologySnapshot(GetTopologySnapshotRequest),
    GetScreenSnapshot(GetScreenSnapshotRequest),
    GetScreenDelta(GetScreenDeltaRequest),
    DispatchMuxCommand(DispatchMuxCommandRequest),
    OpenSubscription(OpenSubscriptionRequest),
}
