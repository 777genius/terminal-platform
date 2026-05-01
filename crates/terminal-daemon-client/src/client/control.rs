use terminal_domain::BackendKind;
use terminal_protocol::{
    BackendCapabilitiesResponse, GetBackendCapabilitiesRequest, Handshake, ProtocolError,
    RequestPayload, ResponsePayload,
};

use crate::HandshakeAssessment;

use super::LocalSocketDaemonClient;

impl LocalSocketDaemonClient {
    pub async fn handshake(&self) -> Result<Handshake, ProtocolError> {
        let response = self.send_request(RequestPayload::Handshake).await?;

        match response.payload {
            ResponsePayload::Handshake(handshake) => Ok(handshake),
            other => Err(ProtocolError::unexpected_payload("handshake", &other)),
        }
    }

    pub async fn handshake_assessment(&self) -> Result<HandshakeAssessment, ProtocolError> {
        let handshake = self.handshake().await?;
        Ok(self.info.assess_handshake(&handshake))
    }
    pub async fn backend_capabilities(
        &self,
        backend: BackendKind,
    ) -> Result<BackendCapabilitiesResponse, ProtocolError> {
        let response = self
            .send_request(RequestPayload::GetBackendCapabilities(GetBackendCapabilitiesRequest {
                backend,
            }))
            .await?;

        match response.payload {
            ResponsePayload::BackendCapabilities(capabilities) => Ok(capabilities),
            other => Err(ProtocolError::unexpected_payload("backend_capabilities", &other)),
        }
    }
}
