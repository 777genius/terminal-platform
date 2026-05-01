use std::{io, sync::Arc};

use futures_util::{SinkExt as _, StreamExt as _};
use interprocess::local_socket::tokio::Stream;
use terminal_protocol::{
    RequestEnvelope, ResponseEnvelope, ResponsePayload, SubscriptionEnvelope, SubscriptionRequest,
    SubscriptionRequestEnvelope, TransportResponse, decode_json_frame, encode_json_frame,
};
use tokio::sync::watch;
use tokio_util::bytes::Bytes;
use tokio_util::codec::{Framed, LengthDelimitedCodec};

use crate::{TransportRequestHandler, TransportSubscriptionHandler};

type LocalFramedStream = Framed<Stream, LengthDelimitedCodec>;

pub(super) async fn handle_connection<Handler>(
    handler: Arc<Handler>,
    stream: Stream,
    mut shutdown_rx: watch::Receiver<bool>,
) -> io::Result<()>
where
    Handler: TransportRequestHandler + TransportSubscriptionHandler + Send + Sync + 'static,
{
    let mut framed = Framed::new(stream, LengthDelimitedCodec::new());

    loop {
        tokio::select! {
            _ = shutdown_rx.changed() => break Ok(()),
            frame_result = framed.next() => {
                let Some(frame_result) = frame_result else {
                    break Ok(());
                };
                let frame = frame_result?;
                let reply = match decode_json_frame::<RequestEnvelope>(&frame) {
                    Ok(request) => {
                        if matches!(&request.payload, terminal_protocol::RequestPayload::OpenSubscription(_)) {
                            return handle_subscription_connection(handler, request, framed, shutdown_rx).await;
                        }
                        TransportResponse::from_result(handler.handle_request(request).await)
                    }
                    Err(error) => TransportResponse::Error(error),
                };
                let encoded_reply = encode_transport_response(&reply)?;

                framed.send(encoded_reply).await?;
            }
        }
    }
}

async fn handle_subscription_connection<Handler>(
    handler: Arc<Handler>,
    request: RequestEnvelope,
    mut framed: LocalFramedStream,
    mut shutdown_rx: watch::Receiver<bool>,
) -> io::Result<()>
where
    Handler: TransportRequestHandler + TransportSubscriptionHandler + Send + Sync + 'static,
{
    let terminal_protocol::RequestPayload::OpenSubscription(open_request) = request.payload else {
        return Err(io::Error::other("subscription connection requires open_subscription request"));
    };
    let mut subscription = handler
        .open_subscription(open_request)
        .await
        .map_err(|error| io::Error::other(error.to_string()))?;
    let subscription_id = subscription.subscription_id;

    send_subscription_opened(&mut framed, request.operation_id, subscription_id).await?;

    let result = loop {
        tokio::select! {
            biased;
            _ = shutdown_rx.changed() => break Ok(()),
            inbound = framed.next() => {
                match inbound {
                    Some(Ok(frame)) => {
                        if let Err(error) = handle_subscription_control_frame(&frame, subscription_id) {
                            break Err(error);
                        }
                        break Ok(());
                    }
                    Some(Err(error)) => break Err(error),
                    None => break Ok(()),
                }
            }
            event = subscription.events.recv() => {
                let Some(event) = event else {
                    break Ok(());
                };
                let envelope = SubscriptionEnvelope { subscription_id, event };
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

async fn send_subscription_opened(
    framed: &mut LocalFramedStream,
    operation_id: terminal_domain::OperationId,
    subscription_id: terminal_domain::SubscriptionId,
) -> io::Result<()> {
    let opened = ResponseEnvelope {
        operation_id,
        payload: ResponsePayload::SubscriptionOpened(terminal_protocol::OpenSubscriptionResponse {
            subscription_id,
        }),
    };
    let encoded_opened = encode_transport_response(&TransportResponse::Response(Box::new(opened)))?;
    framed.send(encoded_opened).await
}

fn handle_subscription_control_frame(
    frame: &[u8],
    subscription_id: terminal_domain::SubscriptionId,
) -> io::Result<()> {
    let envelope = decode_json_frame::<SubscriptionRequestEnvelope>(frame)
        .map_err(|error| io::Error::other(error.to_string()))?;
    if envelope.subscription_id != subscription_id {
        return Err(io::Error::other("subscription control targeted wrong subscription"));
    }
    match envelope.request {
        SubscriptionRequest::Close => Ok(()),
    }
}

fn encode_transport_response(response: &TransportResponse) -> io::Result<Bytes> {
    encode_json_frame(response).map_err(|error| io::Error::other(error.to_string()))
}
