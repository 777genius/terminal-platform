use std::io;

use terminal_protocol::{
    LocalSocketAddress, OpenSubscriptionRequest, ProtocolError, RequestEnvelope, ResponseEnvelope,
    SubscriptionEvent,
};
use terminal_transport::{
    TransportRequestHandler, TransportSubscription, TransportSubscriptionHandler,
};
use tokio::sync::{mpsc, oneshot};

use crate::TerminalDaemon;

pub struct LocalSocketServerHandle(terminal_transport::LocalSocketServerHandle);

impl LocalSocketServerHandle {
    #[must_use]
    pub fn address(&self) -> &LocalSocketAddress {
        self.0.address()
    }

    pub async fn shutdown(self) -> io::Result<()> {
        self.0.shutdown().await
    }
}

pub fn spawn_local_socket_server(
    daemon: TerminalDaemon,
    address: LocalSocketAddress,
) -> io::Result<LocalSocketServerHandle> {
    terminal_transport::spawn_local_socket_server(daemon, address).map(LocalSocketServerHandle)
}

impl TransportRequestHandler for TerminalDaemon {
    fn handle_request(
        &self,
        request: RequestEnvelope,
    ) -> terminal_transport::TransportBoxFuture<'_, Result<ResponseEnvelope, ProtocolError>> {
        Box::pin(async move { TerminalDaemon::handle_request(self, request).await })
    }
}

impl TransportSubscriptionHandler for TerminalDaemon {
    fn open_subscription(
        &self,
        request: OpenSubscriptionRequest,
    ) -> terminal_transport::TransportBoxFuture<'_, Result<TransportSubscription, ProtocolError>>
    {
        Box::pin(async move { self.open_transport_subscription(request).await })
    }
}

pub(crate) async fn backend_subscription_to_transport(
    mut subscription: terminal_backend_api::BackendSubscription,
) -> TransportSubscription {
    let subscription_id = subscription.subscription_id;
    let (events_tx, events_rx) = mpsc::channel(64);
    let (cancel_tx, mut cancel_rx) = oneshot::channel();

    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = &mut cancel_rx => break,
                event = subscription.events.recv() => {
                    let Some(event) = event else {
                        break;
                    };
                    if events_tx.send(map_subscription_event(event)).await.is_err() {
                        break;
                    }
                }
            }
        }

        subscription.cancel();
    });

    TransportSubscription::new(subscription_id, events_rx, cancel_tx)
}

fn map_subscription_event(
    event: terminal_backend_api::BackendSubscriptionEvent,
) -> SubscriptionEvent {
    match event {
        terminal_backend_api::BackendSubscriptionEvent::TopologySnapshot(snapshot) => {
            SubscriptionEvent::TopologySnapshot(snapshot)
        }
        terminal_backend_api::BackendSubscriptionEvent::ScreenDelta(delta) => {
            SubscriptionEvent::ScreenDelta(delta)
        }
        terminal_backend_api::BackendSubscriptionEvent::SessionHealthSnapshot(health) => {
            SubscriptionEvent::SessionHealthSnapshot(health)
        }
    }
}
