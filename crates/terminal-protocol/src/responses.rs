use serde::{Deserialize, Serialize};

use terminal_backend_api::BackendSessionSummary;

use crate::Handshake;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListSessionsResponse {
    pub sessions: Vec<BackendSessionSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ResponsePayload {
    Handshake(Handshake),
    ListSessions(ListSessionsResponse),
    SubscriptionOpened,
}
