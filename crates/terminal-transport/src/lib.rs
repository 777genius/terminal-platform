mod client;
mod server;

use std::{future::Future, pin::Pin};

use tokio::sync::{mpsc, oneshot};

use terminal_protocol::{
    OpenSubscriptionRequest, ProtocolError, RequestEnvelope, ResponseEnvelope, SubscriptionEvent,
};

pub use client::{LocalSocketTransportClient, LocalSocketTransportSubscription};
pub use server::{LocalSocketServerHandle, spawn_local_socket_server};

pub type TransportBoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait TransportRequestHandler: Send + Sync {
    fn handle_request(
        &self,
        request: RequestEnvelope,
    ) -> TransportBoxFuture<'_, Result<ResponseEnvelope, ProtocolError>>;
}

pub trait TransportSubscriptionHandler: Send + Sync {
    fn open_subscription(
        &self,
        request: OpenSubscriptionRequest,
    ) -> TransportBoxFuture<'_, Result<TransportSubscription, ProtocolError>>;
}

#[derive(Debug)]
pub struct TransportSubscription {
    pub subscription_id: terminal_domain::SubscriptionId,
    pub events: mpsc::Receiver<SubscriptionEvent>,
    cancel_tx: Option<oneshot::Sender<()>>,
}

impl TransportSubscription {
    #[must_use]
    pub fn new(
        subscription_id: terminal_domain::SubscriptionId,
        events: mpsc::Receiver<SubscriptionEvent>,
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

#[cfg(test)]
mod tests;
