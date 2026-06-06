mod control;
mod history;
mod mux;
mod saved_sessions;
mod screen;
mod sessions;
mod subscriptions;

use terminal_protocol::{LocalSocketAddress, ProtocolError, RequestPayload};
use terminal_transport::LocalSocketTransportClient;

use crate::DaemonClientInfo;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalSocketDaemonClient {
    pub(crate) transport: LocalSocketTransportClient,
    pub(crate) info: DaemonClientInfo,
}

impl LocalSocketDaemonClient {
    #[must_use]
    pub fn new(address: LocalSocketAddress) -> Self {
        Self {
            transport: LocalSocketTransportClient::new(address),
            info: DaemonClientInfo::default(),
        }
    }

    #[must_use]
    pub fn address(&self) -> &LocalSocketAddress {
        self.transport.address()
    }

    #[must_use]
    pub fn info(&self) -> &DaemonClientInfo {
        &self.info
    }

    pub(crate) async fn send_request(
        &self,
        payload: RequestPayload,
    ) -> Result<terminal_protocol::ResponseEnvelope, ProtocolError> {
        self.transport.send_request(payload).await
    }
}
