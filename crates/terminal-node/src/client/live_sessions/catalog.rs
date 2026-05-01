use terminal_protocol::ProtocolError;

use crate::{NodeHostClient, dto::*};

impl NodeHostClient {
    pub async fn list_sessions(&self) -> Result<Vec<NodeSessionSummary>, ProtocolError> {
        let listed = self.client.list_sessions().await?;
        Ok(listed.sessions.iter().map(Into::into).collect())
    }

    pub async fn discover_sessions(
        &self,
        backend: NodeBackendKind,
    ) -> Result<Vec<NodeDiscoveredSession>, ProtocolError> {
        let discovered = self.client.discover_sessions((&backend).into()).await?;
        Ok(discovered.sessions.iter().map(Into::into).collect())
    }

    pub async fn backend_capabilities(
        &self,
        backend: NodeBackendKind,
    ) -> Result<NodeBackendCapabilitiesInfo, ProtocolError> {
        let capabilities = self.client.backend_capabilities((&backend).into()).await?;
        Ok((&capabilities).into())
    }

    pub async fn create_native_session(
        &self,
        request: &NodeCreateSessionRequest,
    ) -> Result<NodeSessionSummary, ProtocolError> {
        let created = self
            .client
            .create_session(terminal_domain::BackendKind::Native, request.into())
            .await?;

        Ok((&created.session).into())
    }

    pub async fn import_session(
        &self,
        route: &NodeSessionRoute,
        title: Option<String>,
    ) -> Result<NodeSessionSummary, ProtocolError> {
        let imported = self.client.import_session(route.try_into()?, title).await?;
        Ok((&imported.session).into())
    }
}
