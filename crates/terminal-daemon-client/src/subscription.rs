use terminal_domain::SubscriptionId;
use terminal_protocol::{ProtocolError, SubscriptionEvent};
use terminal_transport::LocalSocketTransportSubscription;

pub struct LocalSocketSubscription {
    pub(crate) inner: LocalSocketTransportSubscription,
}

impl LocalSocketSubscription {
    #[must_use]
    pub fn subscription_id(&self) -> SubscriptionId {
        self.inner.subscription_id()
    }

    pub async fn recv(&mut self) -> Result<Option<SubscriptionEvent>, ProtocolError> {
        self.inner.recv().await
    }

    pub async fn close(&mut self) -> Result<(), ProtocolError> {
        self.inner.close().await
    }
}
