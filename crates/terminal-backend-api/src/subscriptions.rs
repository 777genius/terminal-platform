use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use terminal_domain::{PaneId, SubscriptionId};
use terminal_projection::{ScreenDelta, TopologySnapshot};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SubscriptionSpec {
    SessionTopology,
    PaneSurface { pane_id: PaneId },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendSubscriptionEvent {
    TopologySnapshot(TopologySnapshot),
    ScreenDelta(ScreenDelta),
}

#[derive(Debug)]
pub struct BackendSubscription {
    pub subscription_id: SubscriptionId,
    pub events: mpsc::Receiver<BackendSubscriptionEvent>,
}
