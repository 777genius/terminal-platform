use serde::{Deserialize, Serialize};

use terminal_backend_api::{BackendSessionSummary, DiscoveredSession, MuxCommandResult};
use terminal_domain::SubscriptionId;
use terminal_projection::{ScreenDelta, ScreenSnapshot, TopologySnapshot};

use crate::Handshake;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListSessionsResponse {
    pub sessions: Vec<BackendSessionSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateSessionResponse {
    pub session: BackendSessionSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoverSessionsResponse {
    pub sessions: Vec<DiscoveredSession>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportSessionResponse {
    pub session: BackendSessionSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenSubscriptionResponse {
    pub subscription_id: SubscriptionId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ResponsePayload {
    Handshake(Handshake),
    CreateSession(CreateSessionResponse),
    ListSessions(ListSessionsResponse),
    DiscoverSessions(DiscoverSessionsResponse),
    ImportSession(ImportSessionResponse),
    TopologySnapshot(TopologySnapshot),
    ScreenSnapshot(ScreenSnapshot),
    ScreenDelta(ScreenDelta),
    DispatchMuxCommand(MuxCommandResult),
    SubscriptionOpened(OpenSubscriptionResponse),
}
