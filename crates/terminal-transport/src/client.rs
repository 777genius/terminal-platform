use futures_util::{SinkExt as _, StreamExt as _};
use interprocess::local_socket::{tokio::Stream, traits::tokio::Stream as _};
use terminal_protocol::{
    LocalSocketAddress, OpenSubscriptionRequest, ProtocolError, RequestEnvelope, RequestPayload,
    ResponseEnvelope, ResponsePayload, SubscriptionEnvelope, SubscriptionEvent,
    SubscriptionRequest, SubscriptionRequestEnvelope, TransportResponse, decode_json_frame,
    encode_json_frame,
};
use tokio_util::codec::{Framed, LengthDelimitedCodec};

type LocalFramedStream = Framed<Stream, LengthDelimitedCodec>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalSocketTransportClient {
    address: LocalSocketAddress,
}

pub struct LocalSocketTransportSubscription {
    subscription_id: terminal_domain::SubscriptionId,
    framed: LocalFramedStream,
}

impl LocalSocketTransportSubscription {
    #[must_use]
    pub fn subscription_id(&self) -> terminal_domain::SubscriptionId {
        self.subscription_id
    }

    pub async fn recv(&mut self) -> Result<Option<SubscriptionEvent>, ProtocolError> {
        let Some(frame) = self.framed.next().await else {
            return Ok(None);
        };
        let frame = frame.map_err(|error| ProtocolError::io("receive_failed", &error))?;
        let envelope = decode_json_frame::<SubscriptionEnvelope>(&frame)?;
        if envelope.subscription_id != self.subscription_id {
            return Err(ProtocolError::new(
                "subscription_mismatch",
                format!(
                    "expected subscription {:?}, got {:?}",
                    self.subscription_id, envelope.subscription_id
                ),
            ));
        }

        Ok(Some(envelope.event))
    }

    pub async fn close(&mut self) -> Result<(), ProtocolError> {
        let request = SubscriptionRequestEnvelope {
            subscription_id: self.subscription_id,
            request: SubscriptionRequest::Close,
        };
        let encoded_request = encode_json_frame(&request)?;
        if let Err(error) = self.framed.send(encoded_request).await {
            let error = ProtocolError::io("send_failed", &error);
            if is_subscription_close_disconnect(&error) {
                return Ok(());
            }
            return Err(error);
        }
        loop {
            let frame = match self.framed.next().await.transpose() {
                Ok(frame) => frame,
                Err(error) => {
                    let error = ProtocolError::io("receive_failed", &error);
                    if is_subscription_close_disconnect(&error) {
                        break;
                    }
                    return Err(error);
                }
            };
            let Some(frame) = frame else {
                break;
            };
            let envelope = decode_json_frame::<SubscriptionEnvelope>(&frame)?;
            if envelope.subscription_id != self.subscription_id {
                return Err(ProtocolError::new(
                    "subscription_mismatch",
                    format!(
                        "expected subscription {:?}, got {:?}",
                        self.subscription_id, envelope.subscription_id
                    ),
                ));
            }
        }

        Ok(())
    }
}

impl LocalSocketTransportClient {
    #[must_use]
    pub fn new(address: LocalSocketAddress) -> Self {
        Self { address }
    }

    #[must_use]
    pub fn address(&self) -> &LocalSocketAddress {
        &self.address
    }

    pub async fn send_request(
        &self,
        payload: RequestPayload,
    ) -> Result<ResponseEnvelope, ProtocolError> {
        let operation_id = terminal_domain::OperationId::new();
        let request = RequestEnvelope { operation_id, payload };
        let encoded_request = encode_json_frame(&request)?;
        let mut framed = self.connect_framed().await?;

        framed
            .send(encoded_request)
            .await
            .map_err(|error| ProtocolError::io("send_failed", &error))?;

        let frame = framed
            .next()
            .await
            .ok_or_else(|| ProtocolError::new("unexpected_eof", "daemon closed stream"))?
            .map_err(|error| ProtocolError::io("receive_failed", &error))?;
        let response = decode_json_frame::<TransportResponse>(&frame)?.into_result()?;

        if response.operation_id != operation_id {
            return Err(ProtocolError::new(
                "operation_mismatch",
                format!(
                    "expected response for operation {:?}, got {:?}",
                    operation_id, response.operation_id
                ),
            ));
        }

        Ok(response)
    }

    pub async fn open_subscription(
        &self,
        request: OpenSubscriptionRequest,
    ) -> Result<LocalSocketTransportSubscription, ProtocolError> {
        let operation_id = terminal_domain::OperationId::new();
        let request =
            RequestEnvelope { operation_id, payload: RequestPayload::OpenSubscription(request) };
        let encoded_request = encode_json_frame(&request)?;
        let mut framed = self.connect_framed().await?;

        framed
            .send(encoded_request)
            .await
            .map_err(|error| ProtocolError::io("send_failed", &error))?;
        let Some(frame) = framed.next().await else {
            return Err(ProtocolError::new("unexpected_eof", "daemon closed stream"));
        };
        let frame = frame.map_err(|error| ProtocolError::io("receive_failed", &error))?;
        let response = decode_json_frame::<TransportResponse>(&frame)?.into_result()?;

        if response.operation_id != operation_id {
            return Err(ProtocolError::new(
                "operation_mismatch",
                format!(
                    "expected response for operation {:?}, got {:?}",
                    operation_id, response.operation_id
                ),
            ));
        }

        let subscription_id = match response.payload {
            ResponsePayload::SubscriptionOpened(opened) => opened.subscription_id,
            other => return Err(ProtocolError::unexpected_payload("subscription_opened", &other)),
        };

        Ok(LocalSocketTransportSubscription { subscription_id, framed })
    }

    async fn connect_framed(&self) -> Result<LocalFramedStream, ProtocolError> {
        let stream = Stream::connect(
            self.address
                .to_name()
                .map_err(|error| ProtocolError::io("invalid_socket_name", &error))?,
        )
        .await
        .map_err(|error| ProtocolError::io("connect_failed", &error))?;

        Ok(Framed::new(stream, LengthDelimitedCodec::new()))
    }
}

fn is_subscription_close_disconnect(error: &ProtocolError) -> bool {
    if !matches!(error.code.as_str(), "send_failed" | "receive_failed") {
        return false;
    }

    let message = error.message.to_ascii_lowercase();
    message.contains("broken pipe")
        || message.contains("connection reset")
        || message.contains("not connected")
        || message.contains("unexpected eof")
        || message.contains("the pipe is being closed")
        || message.contains("the handle is invalid")
}
