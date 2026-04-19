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
    SubscriptionEvent, SubscriptionRequest, SubscriptionRequestEnvelope, TransportResponse,
    decode_json_frame, encode_json_frame,
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
    let mut subscription = daemon
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
    let encoded_opened = encode_json_frame(&TransportResponse::Response(Box::new(opened)))
        .map_err(|error| io::Error::other(error.to_string()))?;
    framed.send(encoded_opened).await?;

    let result = loop {
        tokio::select! {
            biased;
            inbound = framed.next() => {
                match inbound {
                    Some(Ok(frame)) => {
                        let envelope = decode_json_frame::<SubscriptionRequestEnvelope>(&frame)
                            .map_err(|error| io::Error::other(error.to_string()));
                        let envelope = match envelope {
                            Ok(envelope) => envelope,
                            Err(error) => break Err(error),
                        };
                        if envelope.subscription_id != subscription_id {
                            break Err(io::Error::other("subscription control targeted wrong subscription"));
                        }
                        match envelope.request {
                            SubscriptionRequest::Close => break Ok(()),
                        }
                    }
                    Some(Err(error)) => break Err(error),
                    None => break Ok(()),
                }
            }
            event = subscription.events.recv() => {
                let Some(event) = event else {
                    break Ok(());
                };
                let envelope =
                    SubscriptionEnvelope { subscription_id, event: map_subscription_event(event) };
                let encoded_event = match encode_json_frame(&envelope) {
                    Ok(encoded_event) => encoded_event,
                    Err(error) => break Err(io::Error::other(error.to_string())),
                };
                if let Err(error) = framed.send(encoded_event).await {
                    break Err(error);
                }
            }
        }
    };

    subscription.cancel();

    result
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
