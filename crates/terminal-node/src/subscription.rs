use std::sync::Arc;

use terminal_daemon_client::LocalSocketDaemonClient;
use terminal_domain::SessionId;
use terminal_protocol::ProtocolError;
use tokio::sync::{Mutex, mpsc, oneshot, watch};

use crate::{NodeSubscriptionEvent, NodeSubscriptionMeta, NodeSubscriptionSpec};

#[derive(Debug, Clone)]
pub struct NodeSubscriptionHandle {
    inner: Arc<NodeSubscriptionInner>,
}

#[derive(Debug)]
struct NodeSubscriptionInner {
    subscription_id: terminal_domain::SubscriptionId,
    events: Mutex<mpsc::Receiver<Result<NodeSubscriptionEvent, ProtocolError>>>,
    close_tx: Mutex<Option<oneshot::Sender<()>>>,
    done_rx: Mutex<watch::Receiver<bool>>,
}

impl Drop for NodeSubscriptionInner {
    fn drop(&mut self) {
        if let Some(close_tx) = self.close_tx.get_mut().take() {
            let _ = close_tx.send(());
        }
    }
}

impl NodeSubscriptionHandle {
    pub(crate) async fn open(
        client: LocalSocketDaemonClient,
        session_id: SessionId,
        spec: &NodeSubscriptionSpec,
    ) -> Result<Self, ProtocolError> {
        let mut subscription = client.open_subscription(session_id, spec.try_into()?).await?;
        let subscription_id = subscription.subscription_id();
        let (events_tx, events_rx) = mpsc::channel(32);
        let (close_tx, mut close_rx) = oneshot::channel();
        let (done_tx, done_rx) = watch::channel(false);

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut close_rx => {
                        let _ = subscription.close().await;
                        break;
                    }
                    next = subscription.recv() => {
                        match next {
                            Ok(Some(event)) => {
                                match forward_subscription_event(
                                    &events_tx,
                                    &mut close_rx,
                                    Ok((&event).into()),
                                )
                                .await
                                {
                                    NodeSubscriptionForward::Forwarded => {}
                                    NodeSubscriptionForward::CloseRequested
                                    | NodeSubscriptionForward::ReceiverDropped => {
                                        let _ = subscription.close().await;
                                        break;
                                    }
                                }
                            }
                            Ok(None) => break,
                            Err(error) => {
                                let _ = forward_subscription_event(
                                    &events_tx,
                                    &mut close_rx,
                                    Err(error),
                                )
                                .await;
                                break;
                            }
                        }
                    }
                }
            }

            let _ = done_tx.send(true);
        });

        Ok(Self {
            inner: Arc::new(NodeSubscriptionInner {
                subscription_id,
                events: Mutex::new(events_rx),
                close_tx: Mutex::new(Some(close_tx)),
                done_rx: Mutex::new(done_rx),
            }),
        })
    }

    #[must_use]
    pub fn meta(&self) -> NodeSubscriptionMeta {
        (&self.inner.subscription_id).into()
    }

    pub async fn next_event(&self) -> Result<Option<NodeSubscriptionEvent>, ProtocolError> {
        let mut events = self.inner.events.lock().await;
        match events.recv().await {
            Some(Ok(event)) => Ok(Some(event)),
            Some(Err(error)) => Err(error),
            None => Ok(None),
        }
    }

    pub async fn close(&self) {
        let mut close_tx = self.inner.close_tx.lock().await;
        if let Some(close_tx) = close_tx.take() {
            let _ = close_tx.send(());
        }
        drop(close_tx);

        let mut done_rx = self.inner.done_rx.lock().await;
        while !*done_rx.borrow() {
            if done_rx.changed().await.is_err() {
                break;
            }
        }
    }
}

enum NodeSubscriptionForward {
    Forwarded,
    CloseRequested,
    ReceiverDropped,
}

async fn forward_subscription_event(
    events_tx: &mpsc::Sender<Result<NodeSubscriptionEvent, ProtocolError>>,
    close_rx: &mut oneshot::Receiver<()>,
    event: Result<NodeSubscriptionEvent, ProtocolError>,
) -> NodeSubscriptionForward {
    tokio::select! {
        _ = close_rx => NodeSubscriptionForward::CloseRequested,
        send_result = events_tx.send(event) => {
            if send_result.is_err() {
                NodeSubscriptionForward::ReceiverDropped
            } else {
                NodeSubscriptionForward::Forwarded
            }
        }
    }
}
