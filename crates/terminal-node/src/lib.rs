mod dto;

use std::fs;

use terminal_daemon_client::LocalSocketDaemonClient;
use terminal_domain::{PaneId, SessionId};
use terminal_mux_domain::PaneTreeNode;
use terminal_protocol::{LocalSocketAddress, ProtocolError};
use ts_rs::{Config, TS};
use uuid::Uuid;

pub use dto::{
    NodeAttachedSession, NodeBackendCapabilities, NodeBackendCapabilitiesInfo, NodeBackendKind,
    NodeBindingVersion, NodeCreateSessionRequest, NodeDaemonCapabilities, NodeDaemonPhase,
    NodeDeleteSavedSessionResult, NodeDiscoveredSession, NodeExternalSessionRef, NodeHandshake,
    NodeHandshakeAssessment, NodeHandshakeAssessmentStatus, NodeHandshakeInfo, NodeMuxCommand,
    NodeMuxCommandResult, NodeNewTabCommand, NodeOverrideLayoutCommand, NodePaneSplit,
    NodePaneTreeNode, NodeProjectionSource, NodeProtocolCompatibility,
    NodeProtocolCompatibilityStatus, NodeProtocolVersion, NodePruneSavedSessionsResult,
    NodeRenameTabCommand, NodeResizePaneCommand, NodeRestoredSession, NodeRouteAuthority,
    NodeSavedSessionCompatibility, NodeSavedSessionCompatibilityStatus, NodeSavedSessionManifest,
    NodeSavedSessionRecord, NodeSavedSessionRestoreSemantics, NodeSavedSessionSummary,
    NodeScreenCursor, NodeScreenDelta, NodeScreenLine, NodeScreenLinePatch, NodeScreenPatch,
    NodeScreenSnapshot, NodeScreenSurface, NodeSendInputCommand, NodeSendPasteCommand,
    NodeSessionRoute, NodeSessionSummary, NodeShellLaunchSpec, NodeSplitDirection,
    NodeSplitPaneCommand, NodeTabSnapshot, NodeTopologySnapshot,
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

    pub async fn dispatch_mux_command(
        &self,
        session_id: &str,
        command: &NodeMuxCommand,
    ) -> Result<NodeMuxCommandResult, ProtocolError> {
        let result =
            self.client.dispatch(parse_session_id(session_id)?, command.try_into()?).await?;
        Ok((&result).into())
    }
}

pub fn export_typescript_bindings() -> std::io::Result<()> {
    fs::create_dir_all("./bindings")?;
    let cfg = Config::default();

    NodeBindingVersion::export_all(&cfg).map_err(export_error)?;
    NodeCreateSessionRequest::export_all(&cfg).map_err(export_error)?;
    NodeHandshakeInfo::export_all(&cfg).map_err(export_error)?;
    NodeSessionSummary::export_all(&cfg).map_err(export_error)?;
    NodeDiscoveredSession::export_all(&cfg).map_err(export_error)?;
    NodeBackendCapabilitiesInfo::export_all(&cfg).map_err(export_error)?;
    NodeSavedSessionSummary::export_all(&cfg).map_err(export_error)?;
    NodeSavedSessionRecord::export_all(&cfg).map_err(export_error)?;
    NodeRestoredSession::export_all(&cfg).map_err(export_error)?;
    NodeTopologySnapshot::export_all(&cfg).map_err(export_error)?;
    NodeScreenSnapshot::export_all(&cfg).map_err(export_error)?;
    NodeScreenDelta::export_all(&cfg).map_err(export_error)?;
    NodeMuxCommand::export_all(&cfg).map_err(export_error)?;
    NodeMuxCommandResult::export_all(&cfg).map_err(export_error)?;
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
    use std::{path::PathBuf, time::Duration};

    use terminal_testing::{
        TmuxServerGuard, daemon_fixture, daemon_fixture_with_state, tmux_daemon_state,
        unique_tmux_session_name, unique_tmux_socket_name,
    };
    use tokio::time::sleep;

    use super::{
        NodeBackendKind, NodeCreateSessionRequest, NodeHostClient, NodeMuxCommand,
        NodeNewTabCommand, NodeSendInputCommand, export_typescript_bindings,
    };

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
        let native_capabilities = node
            .backend_capabilities(NodeBackendKind::Native)
            .await
            .expect("native capabilities should succeed");
        let tmux_capabilities = node
            .backend_capabilities(NodeBackendKind::Tmux)
            .await
            .expect("tmux capabilities should succeed");
        let zellij_capabilities = node
            .backend_capabilities(NodeBackendKind::Zellij)
            .await
            .expect("zellij capabilities should succeed");
        let created = node
            .create_native_session(&cat_launch_request("shell"))
            .await
            .expect("create_native_session should succeed");
        let listed = node.list_sessions().await.expect("list_sessions should succeed");
        let attached =
            node.attach_session(&created.session_id).await.expect("attach_session should succeed");
        let topology = node
            .topology_snapshot(&created.session_id)
            .await
            .expect("topology_snapshot should succeed");
        let focused_pane_id =
            attached.focused_screen.as_ref().expect("focused screen should exist").pane_id.clone();
        let ready_screen =
            wait_for_screen_line(&node, &created.session_id, &focused_pane_id, "ready").await;
        let save = node
            .dispatch_mux_command(&created.session_id, &NodeMuxCommand::SaveSession)
            .await
            .expect("save session should succeed");
        let saved = node.list_saved_sessions().await.expect("list_saved_sessions should succeed");
        let loaded =
            node.saved_session(&created.session_id).await.expect("saved_session should succeed");
        let _input = node
            .dispatch_mux_command(
                &created.session_id,
                &NodeMuxCommand::SendInput(NodeSendInputCommand {
                    pane_id: focused_pane_id.clone(),
                    data: "node host input\r".to_string(),
                }),
            )
            .await
            .expect("send input should succeed");
        let after_input =
            wait_for_screen_line(&node, &created.session_id, &focused_pane_id, "node host input")
                .await;
        let delta = node
            .screen_delta(&created.session_id, &focused_pane_id, ready_screen.sequence)
            .await
            .expect("screen_delta should succeed");
        let new_tab = node
            .dispatch_mux_command(
                &created.session_id,
                &NodeMuxCommand::NewTab(NodeNewTabCommand { title: Some("logs".to_string()) }),
            )
            .await
            .expect("new tab should succeed");
        let topology_after_dispatch = node
            .topology_snapshot(&created.session_id)
            .await
            .expect("topology_snapshot should succeed");
        let restored = node
            .restore_saved_session(&created.session_id)
            .await
            .expect("restore_saved_session should succeed");
        let deleted = node
            .delete_saved_session(&created.session_id)
            .await
            .expect("delete_saved_session should succeed");
        let saved_after_delete =
            node.list_saved_sessions().await.expect("list_saved_sessions should succeed");

        assert!(handshake.assessment.can_use);
        assert_eq!(handshake.handshake.available_backends.len(), 3);
        assert_eq!(native_capabilities.backend, NodeBackendKind::Native);
        assert!(native_capabilities.capabilities.explicit_session_save);
        assert_eq!(tmux_capabilities.backend, NodeBackendKind::Tmux);
        assert!(tmux_capabilities.capabilities.read_only_client_mode);
        assert_eq!(zellij_capabilities.backend, NodeBackendKind::Zellij);
        assert!(!zellij_capabilities.capabilities.tab_create);
        assert!(listed.iter().any(|session| session.session_id == created.session_id));
        assert_eq!(attached.session.session_id, created.session_id);
        assert_eq!(attached.topology.session_id, created.session_id);
        assert_eq!(topology.session_id, created.session_id);
        assert!(!topology.tabs.is_empty());
        assert_eq!(ready_screen.pane_id, focused_pane_id);
        assert!(!save.changed);
        assert!(saved.iter().any(|session| session.session_id == created.session_id));
        assert_eq!(loaded.session_id, created.session_id);
        assert!(loaded.compatibility.can_restore);
        assert_eq!(loaded.launch.as_ref().map(|launch| launch.program.as_str()), Some("/bin/sh"));
        assert!(after_input.sequence >= ready_screen.sequence);
        assert!(after_input.surface.lines.iter().any(|line| line.text.contains("node host input")));
        assert_eq!(delta.pane_id, focused_pane_id);
        assert!(delta.to_sequence >= delta.from_sequence);
        assert!(delta.patch.is_some() || delta.full_replace.is_some());
        assert!(new_tab.changed);
        assert_eq!(topology_after_dispatch.tabs.len(), 2);
        assert_eq!(restored.saved_session_id, created.session_id);
        assert_ne!(restored.session.session_id, created.session_id);
        assert_eq!(deleted.session_id, created.session_id);
        assert!(!saved_after_delete.iter().any(|session| session.session_id == created.session_id));

        fixture.shutdown().await.expect("fixture should stop cleanly");
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread")]
    async fn discovers_and_imports_tmux_sessions_through_node_surface() {
        let socket_name = unique_tmux_socket_name("terminal-node-tmux");
        let session_name = unique_tmux_session_name("workspace");
        let _tmux =
            TmuxServerGuard::spawn(&socket_name, &session_name).expect("tmux server should start");
        let fixture =
            daemon_fixture_with_state("terminal-node-tmux", tmux_daemon_state(&socket_name))
                .expect("fixture should start");
        let node = NodeHostClient::new(fixture.client.address().clone());

        let discovered = node
            .discover_sessions(NodeBackendKind::Tmux)
            .await
            .expect("discover_sessions should succeed");
        let candidate = discovered.first().expect("tmux session should be discoverable").clone();
        let imported = node
            .import_session(&candidate.route, candidate.title.clone())
            .await
            .expect("import_session should succeed");
        let topology = node
            .topology_snapshot(&imported.session_id)
            .await
            .expect("topology_snapshot should succeed");
        let focused_pane = topology
            .tabs
            .iter()
            .find(|tab| Some(tab.tab_id.as_str()) == topology.focused_tab.as_deref())
            .and_then(|tab| tab.focused_pane.clone())
            .expect("focused pane should exist");
        let screen =
            wait_for_screen_line(&node, &imported.session_id, &focused_pane, "hello from tmux")
                .await;

        assert_eq!(candidate.route.backend, NodeBackendKind::Tmux);
        assert_eq!(imported.route.backend, NodeBackendKind::Tmux);
        assert_eq!(topology.backend_kind, NodeBackendKind::Tmux);
        assert_eq!(topology.tabs.len(), 2);
        assert!(screen.surface.lines.iter().any(|line| line.text.contains("hello from tmux")));

        fixture.shutdown().await.expect("fixture should stop cleanly");
    }

    #[test]
    fn exports_typescript_bindings_for_node_surface() {
        let export_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bindings");
        export_typescript_bindings().expect("bindings export should succeed");

        assert!(export_dir.join("NodeBindingVersion.ts").exists());
        assert!(export_dir.join("NodeHandshakeInfo.ts").exists());
        assert!(export_dir.join("NodeBackendCapabilitiesInfo.ts").exists());
        assert!(export_dir.join("NodeSavedSessionSummary.ts").exists());
        assert!(export_dir.join("NodeScreenDelta.ts").exists());
        assert!(export_dir.join("NodeMuxCommand.ts").exists());
        assert!(export_dir.join("NodeAttachedSession.ts").exists());
        let binding = std::fs::read_to_string(export_dir.join("NodeHandshakeInfo.ts"))
            .expect("handshake binding should be readable");
        assert!(binding.contains("NodeHandshakeInfo"));
    }

    fn cat_launch_request(title: &str) -> NodeCreateSessionRequest {
        NodeCreateSessionRequest {
            title: Some(title.to_string()),
            launch: Some(super::NodeShellLaunchSpec {
                program: "/bin/sh".to_string(),
                args: vec!["-lc".to_string(), "printf 'ready\\n'; exec cat".to_string()],
                cwd: None,
            }),
        }
    }

    async fn wait_for_screen_line(
        node: &NodeHostClient,
        session_id: &str,
        pane_id: &str,
        needle: &str,
    ) -> super::NodeScreenSnapshot {
        for _ in 0..50 {
            let snapshot = node
                .screen_snapshot(session_id, pane_id)
                .await
                .expect("screen_snapshot should succeed");
            if snapshot.surface.lines.iter().any(|line| line.text.contains(needle)) {
                return snapshot;
            }
            sleep(Duration::from_millis(100)).await;
        }

        panic!("screen never contained expected line: {needle}");
    }
}
