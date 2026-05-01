use terminal_daemon_client::LocalSocketDaemonClient;
use terminal_protocol::{LocalSocketAddress, ProtocolError};

use crate::{
    dto::*,
    ids::{focused_pane_id, parse_pane_id, parse_session_id},
    subscription::NodeSubscriptionHandle,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeHostClient {
    client: LocalSocketDaemonClient,
}

impl NodeHostClient {
    #[must_use]
    pub fn new(address: LocalSocketAddress) -> Self {
        Self { client: LocalSocketDaemonClient::new(address) }
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

    pub async fn list_sessions(&self) -> Result<Vec<NodeSessionSummary>, ProtocolError> {
        let listed = self.client.list_sessions().await?;
        Ok(listed.sessions.iter().map(Into::into).collect())
    }

    pub async fn list_saved_sessions(&self) -> Result<Vec<NodeSavedSessionSummary>, ProtocolError> {
        let listed = self.client.list_saved_sessions().await?;
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

    pub async fn saved_session(
        &self,
        session_id: &str,
    ) -> Result<NodeSavedSessionRecord, ProtocolError> {
        let saved = self.client.saved_session(parse_session_id(session_id)?).await?;
        Ok((&saved.session).into())
    }

    pub async fn delete_saved_session(
        &self,
        session_id: &str,
    ) -> Result<NodeDeleteSavedSessionResult, ProtocolError> {
        let deleted = self.client.delete_saved_session(parse_session_id(session_id)?).await?;
        Ok((&deleted).into())
    }

    pub async fn prune_saved_sessions(
        &self,
        keep_latest: usize,
    ) -> Result<NodePruneSavedSessionsResult, ProtocolError> {
        let pruned = self.client.prune_saved_sessions(keep_latest).await?;
        Ok((&pruned).into())
    }

    pub async fn restore_saved_session(
        &self,
        session_id: &str,
    ) -> Result<NodeRestoredSession, ProtocolError> {
        let restored = self.client.restore_saved_session(parse_session_id(session_id)?).await?;
        Ok((&restored).into())
    }

    pub async fn attach_session(
        &self,
        session_id: &str,
    ) -> Result<NodeAttachedSession, ProtocolError> {
        let session_id = parse_session_id(session_id)?;
        let session = self
            .client
            .list_sessions()
            .await?
            .sessions
            .into_iter()
            .find(|session| session.session_id == session_id)
            .ok_or_else(|| {
                ProtocolError::new("session_not_found", format!("unknown session {session_id:?}"))
            })?;
        let health = self.client.session_health_snapshot(session_id).await?;
        let topology = self.client.topology_snapshot(session_id).await?;
        let focused_screen = match focused_pane_id(&topology) {
            Some(pane_id) => Some(self.client.screen_snapshot(session_id, pane_id).await?),
            None => None,
        };

        Ok(NodeAttachedSession {
            session: (&session).into(),
            health: (&health).into(),
            topology: (&topology).into(),
            focused_screen: focused_screen.as_ref().map(Into::into),
        })
    }

    pub async fn session_health_snapshot(
        &self,
        session_id: &str,
    ) -> Result<NodeSessionHealthSnapshot, ProtocolError> {
        let health = self.client.session_health_snapshot(parse_session_id(session_id)?).await?;
        Ok((&health).into())
    }

    pub async fn topology_snapshot(
        &self,
        session_id: &str,
    ) -> Result<NodeTopologySnapshot, ProtocolError> {
        let snapshot = self.client.topology_snapshot(parse_session_id(session_id)?).await?;
        Ok((&snapshot).into())
    }

    pub async fn screen_snapshot(
        &self,
        session_id: &str,
        pane_id: &str,
    ) -> Result<NodeScreenSnapshot, ProtocolError> {
        let snapshot = self
            .client
            .screen_snapshot(parse_session_id(session_id)?, parse_pane_id(pane_id)?)
            .await?;
        Ok((&snapshot).into())
    }

    pub async fn screen_delta(
        &self,
        session_id: &str,
        pane_id: &str,
        from_sequence: u64,
    ) -> Result<NodeScreenDelta, ProtocolError> {
        let delta = self
            .client
            .screen_delta(parse_session_id(session_id)?, parse_pane_id(pane_id)?, from_sequence)
            .await?;
        Ok((&delta).into())
    }

    pub async fn pane_history(
        &self,
        session_id: &str,
        pane_id: &str,
        from_event_seq: Option<i64>,
        max_segments: Option<i64>,
        max_bytes: Option<i64>,
    ) -> Result<NodePaneHistory, ProtocolError> {
        let history = self
            .client
            .pane_history(
                parse_session_id(session_id)?,
                parse_pane_id(pane_id)?,
                from_event_seq,
                max_segments,
                max_bytes,
            )
            .await?;
        Ok((&history).into())
    }

    pub async fn command_history(
        &self,
        session_id: Option<&str>,
        limit: Option<i64>,
    ) -> Result<Vec<NodeCommandHistoryEntry>, ProtocolError> {
        let session_id = session_id.map(parse_session_id).transpose()?;
        let history = self.client.command_history(session_id, limit).await?;
        Ok(history.entries.iter().map(Into::into).collect())
    }

    pub async fn dispatch_mux_command(
        &self,
        session_id: &str,
        command: &NodeMuxCommand,
    ) -> Result<NodeMuxCommandResult, ProtocolError> {
        let result =
            self.client.dispatch(parse_session_id(session_id)?, command.try_into()?).await?;
        Ok((&result).into())
    }

    pub async fn open_subscription(
        &self,
        session_id: &str,
        spec: &NodeSubscriptionSpec,
    ) -> Result<NodeSubscriptionHandle, ProtocolError> {
        NodeSubscriptionHandle::open(self.client.clone(), parse_session_id(session_id)?, spec).await
    }
}
