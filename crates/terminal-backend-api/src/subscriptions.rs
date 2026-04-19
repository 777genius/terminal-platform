use serde::{Deserialize, Serialize};

use terminal_domain::{PaneId, SessionId, SubscriptionId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SubscriptionSpec {
    SessionTopology { session_id: SessionId },
    PaneSurface { pane_id: PaneId },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackendSubscription {
    pub subscription_id: SubscriptionId,
}
