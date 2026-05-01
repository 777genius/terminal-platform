use futures_util::{SinkExt as _, StreamExt as _};
use terminal_protocol::{
    ProtocolError, SubscriptionEnvelope, SubscriptionEvent, SubscriptionRequest,
    SubscriptionRequestEnvelope, decode_json_frame, encode_json_frame,
};

use super::{LocalFramedStream, disconnect::is_subscription_close_disconnect};

pub struct LocalSocketTransportSubscription {
    subscription_id: terminal_domain::SubscriptionId,
    framed: LocalFramedStream,
}

impl LocalSocketTransportSubscription {
    pub(super) fn new(
        subscription_id: terminal_domain::SubscriptionId,
        framed: LocalFramedStream,
    ) -> Self {
        Self { subscription_id, framed }
    }

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
        self.ensure_subscription_id(envelope.subscription_id)?;

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
        self.drain_until_closed().await
    }

    async fn drain_until_closed(&mut self) -> Result<(), ProtocolError> {
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
            self.ensure_subscription_id(envelope.subscription_id)?;
        }

        Ok(())
    }

    fn ensure_subscription_id(
        &self,
        actual: terminal_domain::SubscriptionId,
    ) -> Result<(), ProtocolError> {
        if actual == self.subscription_id {
            return Ok(());
        }

        Err(ProtocolError::new(
            "subscription_mismatch",
            format!("expected subscription {:?}, got {:?}", self.subscription_id, actual),
        ))
    }
}
