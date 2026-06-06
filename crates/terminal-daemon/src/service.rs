use terminal_backend_api::BackendSubscription;
use terminal_persistence::SqliteSessionStore;
use terminal_protocol::{
    OpenSubscriptionRequest, ProtocolError, RequestEnvelope, ResponseEnvelope,
};
use terminal_runtime::TerminalRuntime;
use terminal_transport::TransportSubscription;

use crate::{
    adapters::TerminalRuntimeAdapter,
    application::{
        TerminalDaemonCatalogPort, TerminalDaemonRequestDispatcher,
        TerminalDaemonSubscriptionService,
    },
    composition, transport,
};

pub struct TerminalDaemon {
    runtime: TerminalRuntime,
}

impl Default for TerminalDaemon {
    fn default() -> Self {
        Self::new(composition::default_runtime())
    }
}

impl TerminalDaemon {
    #[must_use]
    pub fn new(runtime: TerminalRuntime) -> Self {
        Self { runtime }
    }

    #[must_use]
    pub fn with_persistence(persistence: SqliteSessionStore) -> Self {
        Self::new(composition::runtime_with_persistence(persistence))
    }

    pub async fn handle_request(
        &self,
        request: RequestEnvelope,
    ) -> Result<ResponseEnvelope, ProtocolError> {
        self.dispatcher().handle_request(request).await
    }

    pub async fn open_subscription(
        &self,
        request: OpenSubscriptionRequest,
    ) -> Result<BackendSubscription, ProtocolError> {
        self.subscription_service().open_backend_subscription(request).await
    }

    #[must_use]
    pub fn handshake(&self) -> terminal_protocol::Handshake {
        self.runtime_adapter().handshake()
    }

    #[must_use]
    pub fn session_count(&self) -> usize {
        self.runtime.session_count()
    }

    pub(crate) async fn open_transport_subscription(
        &self,
        request: OpenSubscriptionRequest,
    ) -> Result<TransportSubscription, ProtocolError> {
        Ok(transport::backend_subscription_to_transport(self.open_subscription(request).await?)
            .await)
    }

    fn runtime_adapter(&self) -> TerminalRuntimeAdapter<'_> {
        TerminalRuntimeAdapter::new(&self.runtime)
    }

    fn dispatcher(
        &self,
    ) -> TerminalDaemonRequestDispatcher<
        TerminalRuntimeAdapter<'_>,
        TerminalRuntimeAdapter<'_>,
        TerminalRuntimeAdapter<'_>,
        TerminalRuntimeAdapter<'_>,
    > {
        let runtime = self.runtime_adapter();
        TerminalDaemonRequestDispatcher::new(
            runtime,
            runtime,
            runtime,
            TerminalDaemonSubscriptionService::new(runtime),
        )
    }

    fn subscription_service(
        &self,
    ) -> TerminalDaemonSubscriptionService<TerminalRuntimeAdapter<'_>> {
        TerminalDaemonSubscriptionService::new(self.runtime_adapter())
    }
}

#[cfg(test)]
mod tests;
