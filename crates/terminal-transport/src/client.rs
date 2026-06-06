mod disconnect;
mod subscription;

use futures_util::{SinkExt as _, StreamExt as _};
use interprocess::local_socket::{tokio::Stream, traits::tokio::Stream as _};
use terminal_protocol::{
    LocalSocketAddress, OpenSubscriptionRequest, ProtocolError, RequestEnvelope, RequestPayload,
    ResponseEnvelope, ResponsePayload, TransportResponse, decode_json_frame, encode_json_frame,
};
use tokio_util::codec::{Framed, LengthDelimitedCodec};

pub use subscription::LocalSocketTransportSubscription;

type LocalFramedStream = Framed<Stream, LengthDelimitedCodec>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalSocketTransportClient {
    address: LocalSocketAddress,
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

        Ok(LocalSocketTransportSubscription::new(subscription_id, framed))
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
