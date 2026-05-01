use terminal_backend_api::{BackendError, BackendSubscription, SubscriptionSpec};
use terminal_domain::SessionId;

use super::TerminalRuntime;

impl TerminalRuntime {
    pub async fn open_subscription(
        &self,
        session_id: SessionId,
        spec: SubscriptionSpec,
    ) -> Result<BackendSubscription, BackendError> {
        self.sessions.open_subscription(session_id, spec).await
    }
}
