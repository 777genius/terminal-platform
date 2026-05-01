use terminal_backend_api::SubscriptionSpec;
use terminal_domain::SessionId;
use terminal_protocol::{OpenSubscriptionRequest, ProtocolError};

use crate::LocalSocketSubscription;

use super::LocalSocketDaemonClient;

impl LocalSocketDaemonClient {
    pub async fn open_subscription(
        &self,
        session_id: SessionId,
        spec: SubscriptionSpec,
    ) -> Result<LocalSocketSubscription, ProtocolError> {
        self.transport
            .open_subscription(OpenSubscriptionRequest { session_id, spec })
            .await
            .map(|inner| LocalSocketSubscription { inner })
    }
}
