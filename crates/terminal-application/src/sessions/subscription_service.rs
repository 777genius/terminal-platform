use terminal_backend_api::{BackendError, BackendSubscription, SubscriptionSpec};
use terminal_domain::SessionId;

use super::runtime::SessionRuntime;

#[derive(Clone, Copy)]
pub(super) struct SessionSubscriptionService<'a> {
    runtime: SessionRuntime<'a>,
}

impl<'a> SessionSubscriptionService<'a> {
    pub(super) fn new(runtime: SessionRuntime<'a>) -> Self {
        Self { runtime }
    }

    pub(super) async fn open_subscription(
        &self,
        session_id: SessionId,
        spec: SubscriptionSpec,
    ) -> Result<BackendSubscription, BackendError> {
        let session = self.runtime.attach_session(session_id).await?;
        session.subscribe(spec).await
    }
}
