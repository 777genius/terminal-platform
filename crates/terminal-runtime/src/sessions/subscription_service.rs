use terminal_backend_api::{
    BackendError, BackendSubscription, BackendSubscriptionEvent, SubscriptionSpec,
};
use terminal_domain::SessionId;
use terminal_projection::{SessionHealthReason, SessionHealthSnapshot};
use tokio::sync::{mpsc, oneshot};

use super::runtime::SessionRuntime;

#[derive(Clone)]
pub(super) struct SessionSubscriptionService<'a> {
    runtime: SessionRuntime<'a>,
}

impl<'a> SessionSubscriptionService<'a> {
    pub(super) fn new(runtime: SessionRuntime<'a>) -> Self {
        Self { runtime }
    }

    pub(super) async fn open_subscription(
        &self,
        session_id: SessionId,
        spec: SubscriptionSpec,
    ) -> Result<BackendSubscription, BackendError> {
        let session = self.runtime.attach_session(session_id).await?;
        let mut subscription = session.subscribe(spec).await?;
        let subscription_id = subscription.subscription_id;
        let (events_tx, events_rx) = mpsc::channel(64);
        let (cancel_tx, mut cancel_rx) = oneshot::channel();
        let registry = self.runtime.registry_handle();

        tokio::spawn(async move {
            let mut closed_explicitly = false;
            let mut emit_stale_on_close = true;
            loop {
                tokio::select! {
                    _ = &mut cancel_rx => {
                        closed_explicitly = true;
                        emit_stale_on_close = false;
                        break;
                    }
                    next = subscription.events.recv() => {
                        match next {
                            Some(BackendSubscriptionEvent::SessionHealthSnapshot(health)) => {
                                registry.update_health(session_id, health.clone());
                                if events_tx.send(BackendSubscriptionEvent::SessionHealthSnapshot(health.clone())).await.is_err() {
                                    emit_stale_on_close = false;
                                    break;
                                }
                                if health.invalidated {
                                    emit_stale_on_close = false;
                                    break;
                                }
                            }
                            Some(event) => {
                                if events_tx.send(event).await.is_err() {
                                    emit_stale_on_close = false;
                                    break;
                                }
                            }
                            None => break,
                        }
                    }
                }
            }

            if !closed_explicitly && emit_stale_on_close {
                let health = SessionHealthSnapshot::stale(
                    session_id,
                    SessionHealthReason::SubscriptionSourceClosed,
                    "session subscription source closed unexpectedly",
                );
                registry.update_health(session_id, health.clone());
                let _ =
                    events_tx.send(BackendSubscriptionEvent::SessionHealthSnapshot(health)).await;
            }

            subscription.cancel();
        });

        Ok(BackendSubscription::new(subscription_id, events_rx, cancel_tx))
    }
}
