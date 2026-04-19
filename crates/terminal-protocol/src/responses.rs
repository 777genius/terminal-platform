use serde::{Deserialize, Serialize};

use terminal_backend_api::{BackendSessionSummary, MuxCommandResult};
use terminal_projection::{ScreenSnapshot, TopologySnapshot};

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
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ResponsePayload {
    Handshake(Handshake),
    CreateSession(CreateSessionResponse),
    ListSessions(ListSessionsResponse),
    TopologySnapshot(TopologySnapshot),
    ScreenSnapshot(ScreenSnapshot),
    DispatchMuxCommand(MuxCommandResult),
    SubscriptionOpened,
}
