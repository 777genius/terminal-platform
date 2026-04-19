use serde::{Deserialize, Serialize};

use terminal_backend_api::{CreateSessionSpec, MuxCommand, SubscriptionSpec};
use terminal_domain::{BackendKind, PaneId, SessionId, SessionRoute};

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
pub struct DiscoverSessionsRequest {
    pub backend: BackendKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetBackendCapabilitiesRequest {
    pub backend: BackendKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportSessionRequest {
    pub route: SessionRoute,
    pub title: Option<String>,
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
pub struct GetSavedSessionRequest {
    pub session_id: SessionId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestoreSavedSessionRequest {
    pub session_id: SessionId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeleteSavedSessionRequest {
    pub session_id: SessionId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RequestPayload {
    Handshake,
    CreateSession(CreateSessionRequest),
    ListSessions,
    ListSavedSessions,
    DiscoverSessions(DiscoverSessionsRequest),
    GetBackendCapabilities(GetBackendCapabilitiesRequest),
    ImportSession(ImportSessionRequest),
    GetSavedSession(GetSavedSessionRequest),
    DeleteSavedSession(DeleteSavedSessionRequest),
    RestoreSavedSession(RestoreSavedSessionRequest),
    GetTopologySnapshot(GetTopologySnapshotRequest),
    GetScreenSnapshot(GetScreenSnapshotRequest),
    GetScreenDelta(GetScreenDeltaRequest),
    DispatchMuxCommand(DispatchMuxCommandRequest),
    OpenSubscription(OpenSubscriptionRequest),
}
