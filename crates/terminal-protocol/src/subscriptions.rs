use serde::{Deserialize, Serialize};
use terminal_domain::SubscriptionId;

use terminal_projection::{ScreenDelta, TopologySnapshot};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SubscriptionEvent {
    TopologySnapshot(TopologySnapshot),
    ScreenDelta(ScreenDelta),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SubscriptionRequest {
    Close,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscriptionRequestEnvelope {
    pub subscription_id: SubscriptionId,
    pub request: SubscriptionRequest,
}
