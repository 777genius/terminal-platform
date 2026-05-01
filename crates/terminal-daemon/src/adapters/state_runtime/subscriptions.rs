use terminal_backend_api::{BackendError, BackendSubscription, SubscriptionSpec};
use terminal_domain::SessionId;

use crate::application::TerminalDaemonSubscriptionPort;

use super::TerminalRuntimeAdapter;

impl TerminalDaemonSubscriptionPort for TerminalRuntimeAdapter<'_> {
    async fn open_subscription(
        &self,
        session_id: SessionId,
        spec: SubscriptionSpec,
    ) -> Result<BackendSubscription, BackendError> {
        self.runtime.open_subscription(session_id, spec).await
    }
}
