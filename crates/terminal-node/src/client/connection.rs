use terminal_protocol::{LocalSocketAddress, ProtocolError};

use super::NodeHostClient;
use crate::dto::*;

impl NodeHostClient {
    #[must_use]
    pub fn new(address: LocalSocketAddress) -> Self {
        Self { client: terminal_daemon_client::LocalSocketDaemonClient::new(address) }
    }

    #[must_use]
    pub fn from_runtime_slug(slug: impl Into<String>) -> Self {
        Self::new(LocalSocketAddress::from_runtime_slug(slug))
    }

    #[must_use]
    pub fn address(&self) -> &LocalSocketAddress {
        self.client.address()
    }

    #[must_use]
    pub fn binding_version(&self) -> NodeBindingVersion {
        NodeBindingVersion::current(&self.client.info().expected_protocol)
    }

    pub async fn handshake_info(&self) -> Result<NodeHandshakeInfo, ProtocolError> {
        let handshake = self.client.handshake().await?;
        let assessment = self.client.info().assess_handshake(&handshake);

        Ok(NodeHandshakeInfo { handshake: (&handshake).into(), assessment: (&assessment).into() })
    }
}
