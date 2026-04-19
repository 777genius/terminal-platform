use std::{io, sync::Arc};

use futures_util::{SinkExt as _, StreamExt as _};
use interprocess::local_socket::{ListenerOptions, tokio::Stream, traits::tokio::Listener as _};
use tokio::{
    sync::oneshot,
    task::{JoinError, JoinHandle},
};
use tokio_util::codec::{Framed, LengthDelimitedCodec};

use terminal_protocol::{
    LocalSocketAddress, RequestEnvelope, ResponseEnvelope, ResponsePayload, SubscriptionEnvelope,
    SubscriptionEvent, TransportResponse, decode_json_frame, encode_json_frame,
};

use crate::TerminalDaemon;

pub struct LocalSocketServerHandle {
    address: LocalSocketAddress,
    shutdown_tx: Option<oneshot::Sender<()>>,
    task: JoinHandle<io::Result<()>>,
}

impl LocalSocketServerHandle {
    #[must_use]
    pub fn address(&self) -> &LocalSocketAddress {
        &self.address
    }

    pub async fn shutdown(mut self) -> io::Result<()> {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }

        self.task.await.map_err(join_error_to_io)?
    }
}

pub fn spawn_local_socket_server(
    daemon: TerminalDaemon,
    address: LocalSocketAddress,
) -> io::Result<LocalSocketServerHandle> {
    let listener =
        ListenerOptions::new().name(address.to_name()?).try_overwrite(true).create_tokio()?;
    let daemon = Arc::new(daemon);
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel();

    let task = tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = &mut shutdown_rx => break Ok(()),
                accept_result = listener.accept() => {
                    let stream = accept_result?;
                    let daemon = Arc::clone(&daemon);

                    tokio::spawn(async move {
                        let _ = handle_connection(daemon, stream).await;
                    });
                }
            }
        }
    });

    Ok(LocalSocketServerHandle { address, shutdown_tx: Some(shutdown_tx), task })
}

async fn handle_connection(daemon: Arc<TerminalDaemon>, stream: Stream) -> io::Result<()> {
    let mut framed = Framed::new(stream, LengthDelimitedCodec::new());

    while let Some(frame_result) = framed.next().await {
        let frame = frame_result?;
        let reply = match decode_json_frame::<RequestEnvelope>(&frame) {
            Ok(request) => {
                if matches!(
                    &request.payload,
                    terminal_protocol::RequestPayload::OpenSubscription(_)
                ) {
                    return handle_subscription_connection(daemon, request, framed).await;
                }
                TransportResponse::from_result(daemon.handle_request(request).await)
            }
            Err(error) => TransportResponse::Error(error),
        };
        let encoded_reply =
            encode_json_frame(&reply).map_err(|error| io::Error::other(error.to_string()))?;

        framed.send(encoded_reply).await?;
    }

    Ok(())
}

async fn handle_subscription_connection(
    daemon: Arc<TerminalDaemon>,
    request: RequestEnvelope,
    mut framed: Framed<Stream, LengthDelimitedCodec>,
) -> io::Result<()> {
    let terminal_protocol::RequestPayload::OpenSubscription(open_request) = request.payload else {
        return Err(io::Error::other("subscription connection requires open_subscription request"));
    };
    let subscription = daemon
        .open_subscription(open_request)
        .await
        .map_err(|error| io::Error::other(error.to_string()))?;
    let subscription_id = subscription.subscription_id;
    let opened = ResponseEnvelope {
        operation_id: request.operation_id,
        payload: ResponsePayload::SubscriptionOpened(terminal_protocol::OpenSubscriptionResponse {
            subscription_id,
        }),
    };
    let encoded_opened = encode_json_frame(&TransportResponse::Response(opened))
        .map_err(|error| io::Error::other(error.to_string()))?;
    framed.send(encoded_opened).await?;

    let mut events = subscription.events;
    while let Some(event) = events.recv().await {
        let envelope =
            SubscriptionEnvelope { subscription_id, event: map_subscription_event(event) };
        let encoded_event =
            encode_json_frame(&envelope).map_err(|error| io::Error::other(error.to_string()))?;
        framed.send(encoded_event).await?;
    }

    Ok(())
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
    }
}

fn join_error_to_io(error: JoinError) -> io::Error {
    io::Error::other(error.to_string())
}
