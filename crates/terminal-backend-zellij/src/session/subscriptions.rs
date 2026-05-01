use terminal_backend_api::{
    BackendError, BackendSubscription, BackendSubscriptionEvent, SubscriptionSpec,
};
use terminal_domain::PaneId;
use terminal_projection::{ProjectionSource, ScreenDelta};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    sync::{mpsc, oneshot},
    time::{self, MissedTickBehavior},
};

use crate::{constants::ZELLIJ_POLL_INTERVAL, rows::ZellijSubscribeEvent};

use super::ZellijAttachedSession;

impl ZellijAttachedSession {
    pub(super) fn open_subscription(
        &self,
        spec: SubscriptionSpec,
    ) -> Result<BackendSubscription, BackendError> {
        match spec {
            SubscriptionSpec::SessionTopology => self.open_topology_subscription(),
            SubscriptionSpec::PaneSurface { pane_id } => {
                self.open_pane_surface_subscription(pane_id)
            }
        }
    }

    fn open_topology_subscription(&self) -> Result<BackendSubscription, BackendError> {
        let subscription_id = terminal_domain::SubscriptionId::new();
        let session = self.clone();
        let initial = session.snapshot()?.topology;
        let (events_tx, events_rx) = mpsc::channel(32);
        let (cancel_tx, mut cancel_rx) = oneshot::channel();

        tokio::spawn(async move {
            if events_tx
                .send(BackendSubscriptionEvent::TopologySnapshot(initial.clone()))
                .await
                .is_err()
            {
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
                        if events_tx.send(BackendSubscriptionEvent::TopologySnapshot(current)).await.is_err() {
                            break;
                        }
                    }
                }
            }
        });

        Ok(BackendSubscription::new(subscription_id, events_rx, cancel_tx))
    }

    fn open_pane_surface_subscription(
        &self,
        pane_id: PaneId,
    ) -> Result<BackendSubscription, BackendError> {
        let subscription_id = terminal_domain::SubscriptionId::new();
        let session = self.clone();
        let pane_target = session.pane_target(pane_id)?;
        let initial = session.screen_snapshot_inner(pane_id)?;
        let (events_tx, events_rx) = mpsc::channel(32);
        let (cancel_tx, mut cancel_rx) = oneshot::channel();

        tokio::spawn(async move {
            if events_tx
                .send(BackendSubscriptionEvent::ScreenDelta(ScreenDelta::full_replace(0, &initial)))
                .await
                .is_err()
            {
                return;
            }

            let mut child =
                match session.backend.spawn_subscribe(&session.target, &pane_target.backend_ref) {
                    Ok(child) => child,
                    Err(_) => return,
                };
            let Some(stdout) = child.stdout.take() else {
                let _ = child.start_kill();
                let _ = child.wait().await;
                return;
            };
            let mut lines = BufReader::new(stdout).lines();
            let mut last = initial;

            loop {
                tokio::select! {
                    _ = &mut cancel_rx => break,
                    next_line = lines.next_line() => {
                        match next_line {
                            Ok(Some(line)) => {
                                if line.trim().is_empty() {
                                    continue;
                                }
                                let event = match serde_json::from_str::<ZellijSubscribeEvent>(&line) {
                                    Ok(event) => event,
                                    Err(_) => break,
                                };
                                match event {
                                    ZellijSubscribeEvent::PaneUpdate { pane_id: updated_pane_ref, viewport, is_initial, .. } => {
                                        if updated_pane_ref != pane_target.backend_ref || is_initial {
                                            continue;
                                        }
                                        let current = match session.screen_snapshot_from_viewport(
                                            pane_id,
                                            viewport,
                                            ProjectionSource::ZellijViewportSubscribe,
                                        ) {
                                            Ok(snapshot) => snapshot,
                                            Err(_) => break,
                                        };
                                        if current.sequence == last.sequence {
                                            continue;
                                        }
                                        let delta = ScreenDelta::between(&last, &current);
                                        last = current;
                                        if events_tx.send(BackendSubscriptionEvent::ScreenDelta(delta)).await.is_err() {
                                            break;
                                        }
                                    }
                                    ZellijSubscribeEvent::PaneClosed { pane_id: closed_pane_ref } => {
                                        if closed_pane_ref == pane_target.backend_ref {
                                            break;
                                        }
                                    }
                                }
                            }
                            Ok(None) => break,
                            Err(_) => break,
                        }
                    }
                }
            }

            let _ = child.start_kill();
            let _ = child.wait().await;
        });

        Ok(BackendSubscription::new(subscription_id, events_rx, cancel_tx))
    }
}
