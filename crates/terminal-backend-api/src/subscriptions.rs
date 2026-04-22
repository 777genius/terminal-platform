use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};

use terminal_domain::{PaneId, SubscriptionId};
use terminal_projection::{ScreenDelta, SessionHealthSnapshot, TopologySnapshot};

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
    SessionHealthSnapshot(SessionHealthSnapshot),
}

#[derive(Debug)]
pub struct BackendSubscription {
    pub subscription_id: SubscriptionId,
    pub events: mpsc::Receiver<BackendSubscriptionEvent>,
    cancel_tx: Option<oneshot::Sender<()>>,
}

impl BackendSubscription {
    #[must_use]
    pub fn new(
        subscription_id: SubscriptionId,
        events: mpsc::Receiver<BackendSubscriptionEvent>,
        cancel_tx: oneshot::Sender<()>,
    ) -> Self {
        Self { subscription_id, events, cancel_tx: Some(cancel_tx) }
    }

    pub fn cancel(&mut self) {
        if let Some(cancel_tx) = self.cancel_tx.take() {
            let _ = cancel_tx.send(());
        }
    }
}
