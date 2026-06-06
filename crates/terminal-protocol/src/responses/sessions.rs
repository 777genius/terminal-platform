use serde::{Deserialize, Serialize};

use terminal_backend_api::{BackendCapabilities, BackendSessionSummary, DiscoveredSession};
use terminal_domain::{BackendKind, SubscriptionId};

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
pub struct BackendCapabilitiesResponse {
    pub backend: BackendKind,
    pub capabilities: BackendCapabilities,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportSessionResponse {
    pub session: BackendSessionSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenSubscriptionResponse {
    pub subscription_id: SubscriptionId,
}
