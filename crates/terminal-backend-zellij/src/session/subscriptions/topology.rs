use terminal_backend_api::{BackendError, BackendSubscription, BackendSubscriptionEvent};
use tokio::{
    sync::{mpsc, oneshot},
    time::{self, MissedTickBehavior},
};

use crate::constants::ZELLIJ_POLL_INTERVAL;

use super::super::ZellijAttachedSession;

pub(super) fn open_topology_subscription(
    session: &ZellijAttachedSession,
) -> Result<BackendSubscription, BackendError> {
    let subscription_id = terminal_domain::SubscriptionId::new();
    let session = session.clone();
    let initial = session.snapshot()?.topology;
    let (events_tx, events_rx) = mpsc::channel(32);
    let (cancel_tx, cancel_rx) = oneshot::channel();

    tokio::spawn(stream_topology_snapshots(session, initial, events_tx, cancel_rx));

    Ok(BackendSubscription::new(subscription_id, events_rx, cancel_tx))
}

async fn stream_topology_snapshots(
    session: ZellijAttachedSession,
    initial: terminal_projection::TopologySnapshot,
    events_tx: mpsc::Sender<BackendSubscriptionEvent>,
    mut cancel_rx: oneshot::Receiver<()>,
) {
    if send_topology_snapshot(&events_tx, initial.clone()).await.is_err() {
        return;
    }

    let mut last = initial;
    let mut ticker = time::interval(ZELLIJ_POLL_INTERVAL);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = &mut cancel_rx => break,
            _ = ticker.tick() => {
                let Ok(command_idle) = session.command_lane.try_lock() else {
                    continue;
                };
                drop(command_idle);

                let current = match session.snapshot() {
                    Ok(snapshot) => snapshot.topology,
                    Err(_) => break,
                };
                if current == last {
                    continue;
                }

                last = current.clone();
                if send_topology_snapshot(&events_tx, current).await.is_err() {
                    break;
                }
            }
        }
    }
}

async fn send_topology_snapshot(
    events_tx: &mpsc::Sender<BackendSubscriptionEvent>,
    snapshot: terminal_projection::TopologySnapshot,
) -> Result<(), mpsc::error::SendError<BackendSubscriptionEvent>> {
    events_tx.send(BackendSubscriptionEvent::TopologySnapshot(snapshot)).await
}
