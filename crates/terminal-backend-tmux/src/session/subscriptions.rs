use super::TmuxAttachedSession;
use crate::TMUX_POLL_INTERVAL;
use crate::prelude::*;

impl TmuxAttachedSession {
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
            let mut ticker = time::interval(TMUX_POLL_INTERVAL);
            ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

            loop {
                tokio::select! {
                    _ = &mut cancel_rx => break,
                    _ = ticker.tick() => {
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

            let mut last = initial;
            let mut ticker = time::interval(TMUX_POLL_INTERVAL);
            ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

            loop {
                tokio::select! {
                    _ = &mut cancel_rx => break,
                    _ = ticker.tick() => {
                        let current = match session.screen_snapshot_inner(pane_id) {
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
                }
            }
        });

        Ok(BackendSubscription::new(subscription_id, events_rx, cancel_tx))
    }
}
