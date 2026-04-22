use std::sync::Arc;

use terminal_backend_api::{
    BackendError, BackendSubscription, BackendSubscriptionEvent, SubscriptionSpec,
};
use terminal_domain::{PaneId, SubscriptionId};
use tokio::sync::{mpsc, oneshot};

use crate::engine::NativeSessionEngine;

pub(crate) fn open_native_subscription(
    runtime: Arc<NativeSessionEngine>,
    spec: SubscriptionSpec,
) -> Result<BackendSubscription, BackendError> {
    match spec {
        SubscriptionSpec::SessionTopology => open_topology_subscription(runtime),
        SubscriptionSpec::PaneSurface { pane_id } => {
            open_pane_surface_subscription(runtime, pane_id)
        }
    }
}

fn open_topology_subscription(
    runtime: Arc<NativeSessionEngine>,
) -> Result<BackendSubscription, BackendError> {
    let subscription_id = SubscriptionId::new();
    let initial = runtime.topology_snapshot()?;
    let mut topology_tick = runtime.subscribe_topology();
    let (events_tx, events_rx) = mpsc::channel(32);
    let (cancel_tx, mut cancel_rx) = oneshot::channel();

    tokio::spawn(async move {
        if events_tx.send(BackendSubscriptionEvent::TopologySnapshot(initial)).await.is_err() {
            return;
        }

        loop {
            tokio::select! {
                _ = &mut cancel_rx => break,
                changed = topology_tick.changed() => {
                    if changed.is_err() {
                        break;
                    }
                    let snapshot = match runtime.topology_snapshot() {
                        Ok(snapshot) => snapshot,
                        Err(_) => break,
                    };
                    if events_tx.send(BackendSubscriptionEvent::TopologySnapshot(snapshot)).await.is_err() {
                        break;
                    }
                }
            }
        }
    });

    Ok(BackendSubscription::new(subscription_id, events_rx, cancel_tx))
}

fn open_pane_surface_subscription(
    runtime: Arc<NativeSessionEngine>,
    pane_id: PaneId,
) -> Result<BackendSubscription, BackendError> {
    let subscription_id = SubscriptionId::new();
    let initial = runtime.screen_snapshot(pane_id)?;
    let mut last_sequence = initial.sequence;
    let mut surface_tick = runtime.subscribe_pane_surface(pane_id)?;
    let (events_tx, events_rx) = mpsc::channel(32);
    let (cancel_tx, mut cancel_rx) = oneshot::channel();

    tokio::spawn(async move {
        if events_tx
            .send(BackendSubscriptionEvent::ScreenDelta(
                terminal_projection::ScreenDelta::full_replace(0, &initial),
            ))
            .await
            .is_err()
        {
            return;
        }

        loop {
            tokio::select! {
                _ = &mut cancel_rx => break,
                changed = surface_tick.changed() => {
                    if changed.is_err() {
                        break;
                    }
                    let delta = match runtime.screen_delta(pane_id, last_sequence) {
                        Ok(delta) => delta,
                        Err(_) => break,
                    };
                    if delta.to_sequence == last_sequence {
                        continue;
                    }
                    last_sequence = delta.to_sequence;
                    if events_tx.send(BackendSubscriptionEvent::ScreenDelta(delta)).await.is_err() {
                        break;
                    }
                }
            }
        }
    });

    Ok(BackendSubscription::new(subscription_id, events_rx, cancel_tx))
}
