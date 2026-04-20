mod dto;

use std::fs;

use terminal_daemon_client::LocalSocketDaemonClient;
use terminal_domain::{PaneId, SessionId};
use terminal_mux_domain::PaneTreeNode;
use terminal_protocol::{LocalSocketAddress, ProtocolError};
use ts_rs::{Config, TS};
use uuid::Uuid;

pub use dto::{
    NodeAttachedSession, NodeBackendKind, NodeBindingVersion, NodeCreateSessionRequest,
    NodeDaemonCapabilities, NodeDaemonPhase, NodeExternalSessionRef, NodeHandshake,
    NodeHandshakeAssessment, NodeHandshakeAssessmentStatus, NodeHandshakeInfo, NodePaneSplit,
    NodePaneTreeNode, NodeProjectionSource, NodeProtocolCompatibility,
    NodeProtocolCompatibilityStatus, NodeProtocolVersion, NodeRouteAuthority, NodeScreenCursor,
    NodeScreenLine, NodeScreenSnapshot, NodeScreenSurface, NodeSessionRoute, NodeSessionSummary,
    NodeShellLaunchSpec, NodeSplitDirection, NodeTabSnapshot, NodeTopologySnapshot,
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
        let topology = self.client.topology_snapshot(session_id).await?;
        let focused_screen = match focused_pane_id(&topology) {
            Some(pane_id) => Some(self.client.screen_snapshot(session_id, pane_id).await?),
            None => None,
        };

        Ok(NodeAttachedSession {
            session: (&session).into(),
            topology: (&topology).into(),
            focused_screen: focused_screen.as_ref().map(Into::into),
        })
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
}

pub fn export_typescript_bindings() -> std::io::Result<()> {
    fs::create_dir_all("./bindings")?;
    let cfg = Config::default();

    NodeBindingVersion::export_all(&cfg).map_err(export_error)?;
    NodeCreateSessionRequest::export_all(&cfg).map_err(export_error)?;
    NodeHandshakeInfo::export_all(&cfg).map_err(export_error)?;
    NodeSessionSummary::export_all(&cfg).map_err(export_error)?;
    NodeTopologySnapshot::export_all(&cfg).map_err(export_error)?;
    NodeScreenSnapshot::export_all(&cfg).map_err(export_error)?;
    NodeAttachedSession::export_all(&cfg).map_err(export_error)?;

    Ok(())
}

fn export_error(error: ts_rs::ExportError) -> std::io::Error {
    std::io::Error::other(error)
}

fn parse_session_id(value: &str) -> Result<SessionId, ProtocolError> {
    parse_uuid(value, "invalid_session_id", "session").map(SessionId::from)
}

fn parse_pane_id(value: &str) -> Result<PaneId, ProtocolError> {
    parse_uuid(value, "invalid_pane_id", "pane").map(PaneId::from)
}

fn parse_uuid(value: &str, code: &str, label: &str) -> Result<Uuid, ProtocolError> {
    Uuid::parse_str(value).map_err(|error| {
        ProtocolError::new(code, format!("failed to parse {label} id '{value}' - {error}"))
    })
}

fn focused_pane_id(topology: &terminal_projection::TopologySnapshot) -> Option<PaneId> {
    let tab = topology
        .focused_tab
        .and_then(|focused_tab| topology.tabs.iter().find(|tab| tab.tab_id == focused_tab))
        .or_else(|| topology.tabs.first())?;

    tab.focused_pane.or_else(|| first_pane_id(&tab.root))
}

fn first_pane_id(root: &PaneTreeNode) -> Option<PaneId> {
    match root {
        PaneTreeNode::Leaf { pane_id } => Some(*pane_id),
        PaneTreeNode::Split(split) => {
            first_pane_id(&split.first).or_else(|| first_pane_id(&split.second))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use terminal_testing::daemon_fixture;

    use super::{NodeCreateSessionRequest, NodeHostClient, export_typescript_bindings};

    #[test]
    fn exposes_binding_version_from_protocol_contract() {
        let client = NodeHostClient::from_runtime_slug("terminal-node-binding-version");
        let version = client.binding_version();

        assert_eq!(version.binding_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(version.protocol.major, 0);
        assert_eq!(version.protocol.minor, 1);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn roundtrips_node_host_flow_against_daemon_fixture() {
        let fixture = daemon_fixture("terminal-node-host").expect("fixture should start");
        let node = NodeHostClient::new(fixture.client.address().clone());

        let handshake = node.handshake_info().await.expect("handshake_info should succeed");
        let created = node
            .create_native_session(&NodeCreateSessionRequest {
                title: Some("shell".to_string()),
                launch: None,
            })
            .await
            .expect("create_native_session should succeed");
        let listed = node.list_sessions().await.expect("list_sessions should succeed");
        let attached =
            node.attach_session(&created.session_id).await.expect("attach_session should succeed");
        let topology = node
            .topology_snapshot(&created.session_id)
            .await
            .expect("topology_snapshot should succeed");
        let focused_screen = node
            .screen_snapshot(
                &created.session_id,
                attached
                    .focused_screen
                    .as_ref()
                    .expect("focused screen should exist")
                    .pane_id
                    .as_str(),
            )
            .await
            .expect("screen_snapshot should succeed");

        assert!(handshake.assessment.can_use);
        assert_eq!(handshake.handshake.available_backends.len(), 3);
        assert!(listed.iter().any(|session| session.session_id == created.session_id));
        assert_eq!(attached.session.session_id, created.session_id);
        assert_eq!(attached.topology.session_id, created.session_id);
        assert_eq!(topology.session_id, created.session_id);
        assert!(!topology.tabs.is_empty());
        assert_eq!(
            focused_screen.pane_id,
            attached.focused_screen.expect("focused screen should exist").pane_id
        );
        assert!(!focused_screen.surface.lines.is_empty());

        fixture.shutdown().await.expect("fixture should stop cleanly");
    }

    #[test]
    fn exports_typescript_bindings_for_node_surface() {
        let export_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bindings");
        export_typescript_bindings().expect("bindings export should succeed");

        assert!(export_dir.join("NodeBindingVersion.ts").exists());
        assert!(export_dir.join("NodeHandshakeInfo.ts").exists());
        assert!(export_dir.join("NodeAttachedSession.ts").exists());
        let binding = std::fs::read_to_string(export_dir.join("NodeHandshakeInfo.ts"))
            .expect("handshake binding should be readable");
        assert!(binding.contains("NodeHandshakeInfo"));
    }
}
