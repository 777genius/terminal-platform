use serde::{Deserialize, Serialize};

use terminal_projection::{ScreenDelta, TopologySnapshot};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SubscriptionEvent {
    TopologySnapshot(TopologySnapshot),
    ScreenDelta(ScreenDelta),
}
