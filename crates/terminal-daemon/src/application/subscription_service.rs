use terminal_backend_api::BackendSubscription;
use terminal_protocol::{OpenSubscriptionRequest, OpenSubscriptionResponse, ProtocolError};

use crate::adapters::map_backend_error;

use super::TerminalDaemonSubscriptionPort;

pub struct TerminalDaemonSubscriptionService<Subscriptions> {
    subscriptions: Subscriptions,
}

impl<Subscriptions> TerminalDaemonSubscriptionService<Subscriptions> {
    #[must_use]
    pub fn new(subscriptions: Subscriptions) -> Self {
        Self { subscriptions }
    }
}

impl<Subscriptions> TerminalDaemonSubscriptionService<Subscriptions>
where
    Subscriptions: TerminalDaemonSubscriptionPort,
{
    pub async fn open_backend_subscription(
        &self,
        request: OpenSubscriptionRequest,
    ) -> Result<BackendSubscription, ProtocolError> {
        self.subscriptions
            .open_subscription(request.session_id, request.spec)
            .await
            .map_err(map_backend_error)
    }

    pub async fn open_subscription_response(
        &self,
        request: OpenSubscriptionRequest,
    ) -> Result<OpenSubscriptionResponse, ProtocolError> {
        Ok(OpenSubscriptionResponse {
            subscription_id: self.open_backend_subscription(request).await?.subscription_id,
        })
    }
}
