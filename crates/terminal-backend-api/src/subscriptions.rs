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
    ScreenDelta(Box<ScreenDelta>),
    SessionHealthSnapshot(SessionHealthSnapshot),
}

impl BackendSubscriptionEvent {
    #[must_use]
    pub fn screen_delta(delta: ScreenDelta) -> Self {
        Self::ScreenDelta(Box::new(delta))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendRawOutputEvent {
    Bytes(BackendRawOutputBytes),
    Gap(BackendRawOutputGap),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendRawOutputBytes {
    pub pane_id: PaneId,
    pub sequence: u64,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendRawOutputGap {
    pub pane_id: PaneId,
    pub skipped_events: u64,
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

#[derive(Debug)]
pub struct BackendRawOutputSubscription {
    pub subscription_id: SubscriptionId,
    pub events: mpsc::Receiver<BackendRawOutputEvent>,
    cancel_tx: Option<oneshot::Sender<()>>,
}

impl BackendRawOutputSubscription {
    #[must_use]
    pub fn new(
        subscription_id: SubscriptionId,
        events: mpsc::Receiver<BackendRawOutputEvent>,
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
