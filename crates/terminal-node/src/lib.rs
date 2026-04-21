mod dto;

use std::{fs, sync::Arc};

use terminal_daemon_client::LocalSocketDaemonClient;
use terminal_domain::{PaneId, SessionId};
use terminal_mux_domain::PaneTreeNode;
use terminal_protocol::{LocalSocketAddress, ProtocolError};
use tokio::sync::{Mutex, mpsc, oneshot, watch};
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
    NodeSplitPaneCommand, NodeSubscriptionEvent, NodeSubscriptionMeta, NodeSubscriptionSpec,
    NodeTabSnapshot, NodeTopologySnapshot,
};

#[derive(Debug, Clone)]
pub struct NodeSubscriptionHandle {
    inner: Arc<NodeSubscriptionInner>,
}

#[derive(Debug)]
struct NodeSubscriptionInner {
    subscription_id: terminal_domain::SubscriptionId,
    events: Mutex<mpsc::Receiver<Result<NodeSubscriptionEvent, ProtocolError>>>,
    close_tx: Mutex<Option<oneshot::Sender<()>>>,
    done_rx: Mutex<watch::Receiver<bool>>,
}

impl Drop for NodeSubscriptionInner {
    fn drop(&mut self) {
        if let Some(close_tx) = self.close_tx.get_mut().take() {
            let _ = close_tx.send(());
        }
    }
}

impl NodeSubscriptionHandle {
    async fn open(
        client: LocalSocketDaemonClient,
        session_id: SessionId,
        spec: &NodeSubscriptionSpec,
    ) -> Result<Self, ProtocolError> {
        let mut subscription = client.open_subscription(session_id, spec.try_into()?).await?;
        let subscription_id = subscription.subscription_id();
        let (events_tx, events_rx) = mpsc::channel(32);
        let (close_tx, mut close_rx) = oneshot::channel();
        let (done_tx, done_rx) = watch::channel(false);

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut close_rx => {
                        let _ = subscription.close().await;
                        break;
                    }
                    next = subscription.recv() => {
                        match next {
                            Ok(Some(event)) => {
                                match forward_subscription_event(
                                    &events_tx,
                                    &mut close_rx,
                                    Ok((&event).into()),
                                )
                                .await
                                {
                                    NodeSubscriptionForward::Forwarded => {}
                                    NodeSubscriptionForward::CloseRequested
                                    | NodeSubscriptionForward::ReceiverDropped => {
                                        let _ = subscription.close().await;
                                        break;
                                    }
                                }
                            }
                            Ok(None) => break,
                            Err(error) => {
                                let _ = forward_subscription_event(
                                    &events_tx,
                                    &mut close_rx,
                                    Err(error),
                                )
                                .await;
                                break;
                            }
                        }
                    }
                }
            }

            let _ = done_tx.send(true);
        });

        Ok(Self {
            inner: Arc::new(NodeSubscriptionInner {
                subscription_id,
                events: Mutex::new(events_rx),
                close_tx: Mutex::new(Some(close_tx)),
                done_rx: Mutex::new(done_rx),
            }),
        })
    }

    #[must_use]
    pub fn meta(&self) -> NodeSubscriptionMeta {
        (&self.inner.subscription_id).into()
    }

    pub async fn next_event(&self) -> Result<Option<NodeSubscriptionEvent>, ProtocolError> {
        let mut events = self.inner.events.lock().await;
        match events.recv().await {
            Some(Ok(event)) => Ok(Some(event)),
            Some(Err(error)) => Err(error),
            None => Ok(None),
        }
    }

    pub async fn close(&self) {
        let mut close_tx = self.inner.close_tx.lock().await;
        if let Some(close_tx) = close_tx.take() {
            let _ = close_tx.send(());
        }
        drop(close_tx);

        let mut done_rx = self.inner.done_rx.lock().await;
        while !*done_rx.borrow() {
            if done_rx.changed().await.is_err() {
                break;
            }
        }
    }
}

enum NodeSubscriptionForward {
    Forwarded,
    CloseRequested,
    ReceiverDropped,
}

async fn forward_subscription_event(
    events_tx: &mpsc::Sender<Result<NodeSubscriptionEvent, ProtocolError>>,
    close_rx: &mut oneshot::Receiver<()>,
    event: Result<NodeSubscriptionEvent, ProtocolError>,
) -> NodeSubscriptionForward {
    tokio::select! {
        _ = close_rx => NodeSubscriptionForward::CloseRequested,
        send_result = events_tx.send(event) => {
            if send_result.is_err() {
                NodeSubscriptionForward::ReceiverDropped
            } else {
                NodeSubscriptionForward::Forwarded
            }
        }
    }
}

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

    pub async fn open_subscription(
        &self,
        session_id: &str,
        spec: &NodeSubscriptionSpec,
    ) -> Result<NodeSubscriptionHandle, ProtocolError> {
        NodeSubscriptionHandle::open(self.client.clone(), parse_session_id(session_id)?, spec).await
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
    NodeSubscriptionSpec::export_all(&cfg).map_err(export_error)?;
    NodeSubscriptionEvent::export_all(&cfg).map_err(export_error)?;
    NodeSubscriptionMeta::export_all(&cfg).map_err(export_error)?;
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

    use terminal_daemon::{TerminalDaemon, spawn_local_socket_server};
    use terminal_daemon_client::LocalSocketDaemonClient;
    use terminal_domain::DegradedModeReason;
    #[cfg(unix)]
    use terminal_testing::{
        TmuxServerGuard, daemon_fixture_with_state, tmux_daemon_state, unique_tmux_session_name,
        unique_tmux_socket_name,
    };
    use terminal_testing::{
        ZellijSessionGuard, ZellijTestLock, daemon_fixture, daemon_state, echo_shell_launch_spec,
        unique_socket_address, unique_zellij_session_name, wait_for_daemon_ready,
    };
    use tokio::time::{sleep, timeout};

    use super::{
        NodeBackendKind, NodeCreateSessionRequest, NodeHostClient, NodeMuxCommand,
        NodeNewTabCommand, NodePaneTreeNode, NodeProjectionSource, NodeRenameTabCommand,
        NodeSendInputCommand, NodeSubscriptionEvent, NodeSubscriptionSpec,
        export_typescript_bindings,
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
        let ready_screen = wait_for_interactive_screen(
            &node,
            &created.session_id,
            &focused_pane_id,
            "node-host-roundtrip",
        )
        .await;
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
                    data: submitted_input("node host input"),
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
        if zellij_capabilities.capabilities.rendered_viewport_snapshot {
            assert!(zellij_capabilities.capabilities.tab_create);
            assert!(zellij_capabilities.capabilities.tab_close);
            assert!(zellij_capabilities.capabilities.tab_focus);
            assert!(zellij_capabilities.capabilities.tab_rename);
            assert!(zellij_capabilities.capabilities.rendered_viewport_stream);
            assert!(zellij_capabilities.capabilities.session_scoped_tab_refs);
            assert!(zellij_capabilities.capabilities.session_scoped_pane_refs);
            assert!(zellij_capabilities.capabilities.pane_close);
            assert!(zellij_capabilities.capabilities.pane_focus);
            assert!(zellij_capabilities.capabilities.pane_input_write);
            assert!(zellij_capabilities.capabilities.pane_paste_write);
            assert!(zellij_capabilities.capabilities.plugin_panes);
            assert!(zellij_capabilities.capabilities.advisory_metadata_subscriptions);
            assert!(zellij_capabilities.capabilities.read_only_client_mode);
        } else {
            assert!(!zellij_capabilities.capabilities.tab_create);
            assert!(!zellij_capabilities.capabilities.tab_close);
            assert!(!zellij_capabilities.capabilities.tab_focus);
            assert!(!zellij_capabilities.capabilities.tab_rename);
            assert!(!zellij_capabilities.capabilities.pane_close);
            assert!(!zellij_capabilities.capabilities.pane_focus);
            assert!(!zellij_capabilities.capabilities.pane_input_write);
            assert!(!zellij_capabilities.capabilities.pane_paste_write);
            assert!(!zellij_capabilities.capabilities.rendered_viewport_stream);
        }
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
        let expected_launch =
            cat_launch_request("shell").launch.expect("cat launch request should include launch");
        assert_eq!(
            loaded.launch.as_ref().map(|launch| launch.program.as_str()),
            Some(expected_launch.program.as_str())
        );
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

    #[cfg(any(unix, windows))]
    #[tokio::test(flavor = "multi_thread")]
    async fn discovers_zellij_sessions_and_handles_import_surface_through_node_surface() {
        let _zellij_lock = ZellijTestLock::acquire().expect("zellij test lock should acquire");
        let attempts = 3;
        let mut last_error = None;

        for attempt in 0..attempts {
            let run = tokio::spawn(async move {
                timeout(zellij_attempt_timeout(), async move {
                    let session_name = unique_zellij_session_name("workspace");
                    let _zellij = ZellijSessionGuard::spawn(&session_name)
                        .expect("zellij session should start");
                    let fixture =
                        daemon_fixture("terminal-node-zellij").expect("fixture should start");
                    let node = NodeHostClient::new(fixture.client.address().clone());
                    let zellij_capabilities = node
                        .backend_capabilities(NodeBackendKind::Zellij)
                        .await
                        .expect("zellij capabilities should succeed");

                    let candidate = wait_for_discovered_zellij_session(&node, &session_name).await;

                    assert_eq!(candidate.route.backend, NodeBackendKind::Zellij);

                    if !zellij_capabilities.capabilities.rendered_viewport_snapshot {
                        let error = timeout(
                            extended_timeout(),
                            node.import_session(&candidate.route, candidate.title.clone()),
                        )
                        .await
                        .expect("import_session should not hang")
                        .expect_err("legacy zellij surface should reject imported attach");

                        assert_eq!(error.code, "backend_unsupported");
                        assert_eq!(
                            error.degraded_reason,
                            Some(DegradedModeReason::MissingCapability)
                        );
                        assert!(error.message.contains("zellij"));
                    } else {
                        let imported = timeout(
                            zellij_operation_timeout(),
                            node.import_session(&candidate.route, candidate.title.clone()),
                        )
                        .await
                        .expect("import_session should not hang")
                        .expect("rich zellij surface should import successfully");
                        let topology = timeout(
                            extended_timeout(),
                            node.topology_snapshot(&imported.session_id),
                        )
                        .await
                        .expect("topology_snapshot should not hang")
                        .expect("topology_snapshot should succeed");
                        let focused_tab = topology
                            .tabs
                            .iter()
                            .find(|tab| {
                                Some(tab.tab_id.as_str()) == topology.focused_tab.as_deref()
                            })
                            .or_else(|| topology.tabs.first())
                            .expect("zellij topology should have tabs");
                        let focused_pane = focused_tab
                            .focused_pane
                            .clone()
                            .or_else(|| first_node_pane_id(&focused_tab.root))
                            .expect("focused zellij pane should exist");
                        let screen = timeout(
                            extended_timeout(),
                            node.screen_snapshot(&imported.session_id, &focused_pane),
                        )
                        .await
                        .expect("screen_snapshot should not hang")
                        .expect("screen_snapshot should succeed");
                        let delta = timeout(
                            extended_timeout(),
                            node.screen_delta(&imported.session_id, &focused_pane, screen.sequence),
                        )
                        .await
                        .expect("screen_delta should not hang")
                        .expect("screen_delta should succeed");
                        let topology_subscription = node
                            .open_subscription(
                                &imported.session_id,
                                &NodeSubscriptionSpec::SessionTopology,
                            )
                            .await
                            .expect("zellij topology subscription should open");
                        let pane_subscription = node
                            .open_subscription(
                                &imported.session_id,
                                &NodeSubscriptionSpec::PaneSurface {
                                    pane_id: focused_pane.clone(),
                                },
                            )
                            .await
                            .expect("zellij pane subscription should open");
                        let initial_topology =
                            timeout(extended_timeout(), topology_subscription.next_event())
                                .await
                                .expect("zellij topology subscription should not hang")
                                .expect("zellij topology subscription should stay healthy")
                                .expect("zellij topology subscription should emit initial event");
                        let initial_pane =
                            timeout(extended_timeout(), pane_subscription.next_event())
                                .await
                                .expect("zellij pane subscription should not hang")
                                .expect("zellij pane subscription should stay healthy")
                                .expect("zellij pane subscription should emit initial event");

                        assert_eq!(imported.route.backend, NodeBackendKind::Zellij);
                        assert_eq!(topology.backend_kind, NodeBackendKind::Zellij);
                        assert!(!topology.tabs.is_empty());
                        assert_eq!(screen.pane_id, focused_pane);
                        assert_eq!(screen.source, NodeProjectionSource::ZellijDumpSnapshot);
                        assert_zellij_delta_compatible_with_snapshot(&screen, &delta);
                        match initial_topology {
                            NodeSubscriptionEvent::TopologySnapshot(snapshot) => {
                                assert_eq!(snapshot.session_id, imported.session_id);
                                assert_eq!(snapshot.backend_kind, NodeBackendKind::Zellij);
                            }
                            other => panic!("unexpected initial zellij topology event: {other:?}"),
                        }
                        match initial_pane {
                            NodeSubscriptionEvent::ScreenDelta(delta) => {
                                assert_eq!(delta.pane_id, focused_pane);
                                assert_eq!(delta.source, NodeProjectionSource::ZellijDumpSnapshot);
                                assert!(delta.full_replace.is_some());
                            }
                            other => panic!("unexpected initial zellij pane event: {other:?}"),
                        }

                        let initial_tab_count = topology.tabs.len();
                        let initial_focused_tab =
                            topology.focused_tab.clone().expect("focused zellij tab should exist");

                        let created = timeout(
                            zellij_operation_timeout(),
                            node.dispatch_mux_command(
                                &imported.session_id,
                                &NodeMuxCommand::NewTab(NodeNewTabCommand {
                                    title: Some("logs-rich".to_string()),
                                }),
                            ),
                        )
                        .await
                        .expect("zellij new_tab should not hang")
                        .expect("zellij new_tab should succeed");
                        let after_create = wait_for_topology_state(
                            &node,
                            &imported.session_id,
                            |snapshot| {
                                snapshot.tabs.len() == initial_tab_count + 1
                                    && snapshot
                                        .tabs
                                        .iter()
                                        .any(|tab| tab.title.as_deref() == Some("logs-rich"))
                            },
                            "zellij rich new tab topology",
                        )
                        .await;
                        let rich_tab_id = after_create
                            .tabs
                            .iter()
                            .find(|tab| tab.title.as_deref() == Some("logs-rich"))
                            .map(|tab| tab.tab_id.clone())
                            .expect("created rich zellij tab should exist");

                        let renamed = timeout(
                            zellij_operation_timeout(),
                            node.dispatch_mux_command(
                                &imported.session_id,
                                &NodeMuxCommand::RenameTab(NodeRenameTabCommand {
                                    tab_id: rich_tab_id.clone(),
                                    title: "logs-rich-renamed".to_string(),
                                }),
                            ),
                        )
                        .await
                        .expect("zellij rename_tab should not hang")
                        .expect("zellij rename_tab should succeed");
                        let after_rename = wait_for_topology_state(
                            &node,
                            &imported.session_id,
                            |snapshot| {
                                snapshot.tabs.iter().any(|tab| {
                                    tab.tab_id == rich_tab_id
                                        && tab.title.as_deref() == Some("logs-rich-renamed")
                                })
                            },
                            "zellij rich renamed tab topology",
                        )
                        .await;

                        let focused = timeout(
                            zellij_operation_timeout(),
                            node.dispatch_mux_command(
                                &imported.session_id,
                                &NodeMuxCommand::FocusTab { tab_id: initial_focused_tab.clone() },
                            ),
                        )
                        .await
                        .expect("zellij focus_tab should not hang")
                        .expect("zellij focus_tab should succeed");
                        let after_focus = wait_for_topology_state(
                            &node,
                            &imported.session_id,
                            |snapshot| {
                                snapshot.focused_tab.as_deref()
                                    == Some(initial_focused_tab.as_str())
                            },
                            "zellij rich focus tab topology",
                        )
                        .await;

                        let closed = timeout(
                            zellij_operation_timeout(),
                            node.dispatch_mux_command(
                                &imported.session_id,
                                &NodeMuxCommand::CloseTab { tab_id: rich_tab_id.clone() },
                            ),
                        )
                        .await
                        .expect("zellij close_tab should not hang")
                        .expect("zellij close_tab should succeed");
                        let after_close = wait_for_topology_state(
                            &node,
                            &imported.session_id,
                            |snapshot| {
                                snapshot.tabs.len() == initial_tab_count
                                    && snapshot.tabs.iter().all(|tab| tab.tab_id != rich_tab_id)
                            },
                            "zellij rich close tab topology",
                        )
                        .await;

                        assert!(created.changed);
                        assert_eq!(after_create.tabs.len(), initial_tab_count + 1);
                        assert!(renamed.changed);
                        assert!(after_rename.tabs.iter().any(|tab| {
                            tab.tab_id == rich_tab_id
                                && tab.title.as_deref() == Some("logs-rich-renamed")
                        }));
                        assert!(focused.changed);
                        assert_eq!(
                            after_focus.focused_tab.as_deref(),
                            Some(initial_focused_tab.as_str())
                        );
                        assert!(closed.changed);
                        assert_eq!(after_close.tabs.len(), initial_tab_count);

                        topology_subscription.close().await;
                        pane_subscription.close().await;
                    }

                    fixture.shutdown().await.expect("fixture should stop cleanly");
                })
                .await
                .expect("zellij node smoke attempt should complete within timeout");
            });

            match run.await {
                Ok(()) => return,
                Err(error) => {
                    last_error = Some(format!("attempt {} failed: {error}", attempt + 1));
                    sleep(Duration::from_millis(250)).await;
                }
            }
        }

        panic!(
            "node zellij import smoke failed after {attempts} attempts: {}",
            last_error.unwrap_or_else(|| "unknown failure".to_string())
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn streams_subscription_events_through_node_surface() {
        let fixture = daemon_fixture("terminal-node-subscriptions").expect("fixture should start");
        let node = NodeHostClient::new(fixture.client.address().clone());
        let created = node
            .create_native_session(&cat_launch_request("shell"))
            .await
            .expect("create_native_session should succeed");
        let attached =
            node.attach_session(&created.session_id).await.expect("attach_session should succeed");
        let pane_id =
            attached.focused_screen.as_ref().expect("focused screen should exist").pane_id.clone();
        wait_for_interactive_screen(
            &node,
            &created.session_id,
            &pane_id,
            "node-host-subscriptions",
        )
        .await;

        let topology_subscription = node
            .open_subscription(&created.session_id, &NodeSubscriptionSpec::SessionTopology)
            .await
            .expect("topology subscription should open");
        let initial_topology = topology_subscription
            .next_event()
            .await
            .expect("initial topology event should arrive")
            .expect("initial topology event should exist");

        let pane_subscription = node
            .open_subscription(
                &created.session_id,
                &NodeSubscriptionSpec::PaneSurface { pane_id: pane_id.clone() },
            )
            .await
            .expect("pane subscription should open");
        let initial_pane = pane_subscription
            .next_event()
            .await
            .expect("initial pane event should arrive")
            .expect("initial pane event should exist");

        node.dispatch_mux_command(
            &created.session_id,
            &NodeMuxCommand::NewTab(NodeNewTabCommand { title: Some("logs".to_string()) }),
        )
        .await
        .expect("new tab should succeed");
        let topology_update = next_topology_snapshot(&topology_subscription)
            .await
            .expect("topology snapshot should arrive");

        node.dispatch_mux_command(
            &created.session_id,
            &NodeMuxCommand::SendInput(NodeSendInputCommand {
                pane_id: pane_id.clone(),
                data: submitted_input("node subscription input"),
            }),
        )
        .await
        .expect("send input should succeed");
        let pane_update = wait_for_subscription_line(&pane_subscription, "node subscription input")
            .await
            .expect("pane update should arrive");

        assert_eq!(
            initial_topology,
            NodeSubscriptionEvent::TopologySnapshot(attached.topology.clone())
        );
        assert!(matches!(
            initial_pane,
            NodeSubscriptionEvent::ScreenDelta(delta) if delta.full_replace.is_some()
        ));
        assert_eq!(topology_update.tabs.len(), 2);
        assert!(subscription_delta_contains(&pane_update, "node subscription input"));

        topology_subscription.close().await;
        pane_subscription.close().await;
        fixture.shutdown().await.expect("fixture should stop cleanly");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn closes_subscription_stream_when_daemon_shuts_down() {
        let fixture = daemon_fixture("terminal-node-close").expect("fixture should start");
        let node = NodeHostClient::new(fixture.client.address().clone());
        let created = node
            .create_native_session(&cat_launch_request("shutdown"))
            .await
            .expect("create_native_session should succeed");
        let attached =
            node.attach_session(&created.session_id).await.expect("attach_session should succeed");
        let pane_id =
            attached.focused_screen.as_ref().expect("focused screen should exist").pane_id.clone();
        let pane_subscription = node
            .open_subscription(
                &created.session_id,
                &NodeSubscriptionSpec::PaneSurface { pane_id: pane_id.clone() },
            )
            .await
            .expect("pane subscription should open");

        let initial = pane_subscription
            .next_event()
            .await
            .expect("initial pane event should arrive")
            .expect("initial pane event should exist");
        assert!(matches!(
            initial,
            NodeSubscriptionEvent::ScreenDelta(delta) if delta.full_replace.is_some()
        ));

        fixture.shutdown().await.expect("fixture should stop cleanly");

        assert!(wait_for_subscription_close(&pane_subscription).await);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn closes_subscription_bridge_under_backpressure() {
        let fixture = daemon_fixture("terminal-node-backpressure").expect("fixture should start");
        let node = NodeHostClient::new(fixture.client.address().clone());
        let created = node
            .create_native_session(&cat_launch_request("backpressure"))
            .await
            .expect("create_native_session should succeed");
        let topology = node
            .topology_snapshot(&created.session_id)
            .await
            .expect("topology_snapshot should succeed");
        let tab_id = topology.focused_tab.clone().expect("focused tab should exist");
        let subscription = node
            .open_subscription(&created.session_id, &NodeSubscriptionSpec::SessionTopology)
            .await
            .expect("topology subscription should open");

        let initial = subscription
            .next_event()
            .await
            .expect("initial topology event should arrive")
            .expect("initial topology event should exist");
        assert!(matches!(initial, NodeSubscriptionEvent::TopologySnapshot(_)));

        for revision in 0..96 {
            node.dispatch_mux_command(
                &created.session_id,
                &NodeMuxCommand::RenameTab(NodeRenameTabCommand {
                    tab_id: tab_id.clone(),
                    title: format!("backpressure-{revision}"),
                }),
            )
            .await
            .expect("rename tab should succeed");
        }

        timeout(operation_timeout(), subscription.close())
            .await
            .expect("subscription close should not hang under backpressure");
        timeout(operation_timeout(), fixture.shutdown())
            .await
            .expect("fixture shutdown should not hang after backpressure close")
            .expect("fixture should stop cleanly");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn repeatedly_reopens_subscriptions_through_node_surface() {
        let fixture = daemon_fixture("terminal-node-reopen").expect("fixture should start");
        let node = NodeHostClient::new(fixture.client.address().clone());
        let created = node
            .create_native_session(&cat_launch_request("reopen"))
            .await
            .expect("create_native_session should succeed");
        let attached =
            node.attach_session(&created.session_id).await.expect("attach_session should succeed");
        let pane_id =
            attached.focused_screen.as_ref().expect("focused screen should exist").pane_id.clone();
        wait_for_interactive_screen(&node, &created.session_id, &pane_id, "node-host-reopen").await;

        for cycle in 0..24 {
            let topology_subscription = timeout(
                operation_timeout(),
                node.open_subscription(&created.session_id, &NodeSubscriptionSpec::SessionTopology),
            )
            .await
            .expect("topology subscription open should not hang")
            .expect("topology subscription should open");
            let initial_topology = timeout(operation_timeout(), topology_subscription.next_event())
                .await
                .expect("topology subscription next_event should not hang")
                .expect("topology subscription should stay healthy")
                .expect("topology subscription should yield initial event");
            assert!(
                matches!(
                    initial_topology,
                    NodeSubscriptionEvent::TopologySnapshot(snapshot)
                        if snapshot.session_id == created.session_id
                ),
                "cycle {cycle} should receive an initial topology snapshot"
            );
            timeout(operation_timeout(), topology_subscription.close())
                .await
                .expect("topology subscription close should not hang");

            let pane_subscription = timeout(
                operation_timeout(),
                node.open_subscription(
                    &created.session_id,
                    &NodeSubscriptionSpec::PaneSurface { pane_id: pane_id.clone() },
                ),
            )
            .await
            .expect("pane subscription open should not hang")
            .expect("pane subscription should open");
            let initial_pane = timeout(operation_timeout(), pane_subscription.next_event())
                .await
                .expect("pane subscription next_event should not hang")
                .expect("pane subscription should stay healthy")
                .expect("pane subscription should yield initial event");
            assert!(
                matches!(
                    initial_pane,
                    NodeSubscriptionEvent::ScreenDelta(delta) if delta.full_replace.is_some()
                ),
                "cycle {cycle} should receive an initial pane delta"
            );

            if cycle % 6 == 5 {
                let marker = format!("node reopen cycle {cycle}");
                node.dispatch_mux_command(
                    &created.session_id,
                    &NodeMuxCommand::SendInput(NodeSendInputCommand {
                        pane_id: pane_id.clone(),
                        data: submitted_input(&marker),
                    }),
                )
                .await
                .expect("send input should succeed during reopen stress");
                let update = timeout(
                    operation_timeout(),
                    wait_for_subscription_line(&pane_subscription, &marker),
                )
                .await
                .expect("pane update wait should not hang")
                .expect("pane update should arrive");
                assert!(
                    subscription_delta_contains(&update, &marker),
                    "cycle {cycle} should receive the live pane update"
                );
            }

            timeout(operation_timeout(), pane_subscription.close())
                .await
                .expect("pane subscription close should not hang");
        }

        let final_list = node.list_sessions().await.expect("list_sessions should succeed");
        assert!(final_list.iter().any(|session| session.session_id == created.session_id));

        fixture.shutdown().await.expect("fixture should stop cleanly");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn recovers_node_host_client_after_daemon_restart() {
        let address = unique_socket_address("terminal-node-restart");
        let readiness_client = LocalSocketDaemonClient::new(address.clone());
        let node = NodeHostClient::new(address.clone());
        let server = spawn_daemon_with_retry(address.clone()).expect("initial daemon should bind");
        wait_for_daemon_ready(&readiness_client).await;

        let initial_list = timeout(operation_timeout(), node.list_sessions())
            .await
            .expect("initial list_sessions should not hang")
            .expect("initial list_sessions should succeed");
        assert!(initial_list.is_empty());

        server.shutdown().await.expect("initial daemon should stop cleanly");

        let stale_result = timeout(operation_timeout(), node.list_sessions())
            .await
            .expect("stale list_sessions should not hang");
        assert!(stale_result.is_err(), "stale daemon request should fail");

        let restarted_readiness_client = LocalSocketDaemonClient::new(address.clone());
        let replacement =
            spawn_daemon_with_retry(address.clone()).expect("replacement daemon should bind");
        wait_for_daemon_ready(&restarted_readiness_client).await;

        let created = timeout(
            operation_timeout(),
            node.create_native_session(&cat_launch_request("restart")),
        )
        .await
        .expect("post-restart create_native_session should not hang")
        .expect("post-restart create_native_session should succeed");
        let attached = timeout(operation_timeout(), node.attach_session(&created.session_id))
            .await
            .expect("post-restart attach_session should not hang")
            .expect("post-restart attach_session should succeed");
        let pane_id = attached
            .focused_screen
            .as_ref()
            .expect("focused screen should exist after restart")
            .pane_id
            .clone();
        let subscription = timeout(
            operation_timeout(),
            node.open_subscription(
                &created.session_id,
                &NodeSubscriptionSpec::PaneSurface { pane_id: pane_id.clone() },
            ),
        )
        .await
        .expect("post-restart subscription open should not hang")
        .expect("post-restart subscription should open");
        let initial_event = timeout(operation_timeout(), subscription.next_event())
            .await
            .expect("post-restart subscription next_event should not hang")
            .expect("post-restart subscription should stay healthy")
            .expect("post-restart subscription should yield an event");

        assert!(matches!(
            initial_event,
            NodeSubscriptionEvent::ScreenDelta(delta) if delta.full_replace.is_some()
        ));

        timeout(operation_timeout(), subscription.close())
            .await
            .expect("post-restart subscription close should not hang");
        replacement.shutdown().await.expect("replacement daemon should stop cleanly");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn recovers_node_host_client_across_multiple_daemon_restart_cycles() {
        let address = unique_socket_address("terminal-node-restart-cycles");
        let node = NodeHostClient::new(address.clone());

        for cycle in 0..3 {
            let readiness_client = LocalSocketDaemonClient::new(address.clone());
            let server = spawn_daemon_with_retry(address.clone())
                .expect("daemon should bind for restart cycle");
            wait_for_daemon_ready(&readiness_client).await;

            let listed = timeout(operation_timeout(), node.list_sessions())
                .await
                .expect("list_sessions should not hang")
                .expect("list_sessions should succeed");
            assert!(listed.is_empty(), "cycle {cycle} should start with a fresh daemon state");

            let created = timeout(
                operation_timeout(),
                node.create_native_session(&cat_launch_request(&format!("restart-cycle-{cycle}"))),
            )
            .await
            .expect("create_native_session should not hang")
            .expect("create_native_session should succeed");
            let attached = timeout(operation_timeout(), node.attach_session(&created.session_id))
                .await
                .expect("attach_session should not hang")
                .expect("attach_session should succeed");
            let pane_id = attached
                .focused_screen
                .as_ref()
                .expect("focused screen should exist after restart cycle")
                .pane_id
                .clone();
            let subscription = timeout(
                operation_timeout(),
                node.open_subscription(
                    &created.session_id,
                    &NodeSubscriptionSpec::PaneSurface { pane_id: pane_id.clone() },
                ),
            )
            .await
            .expect("subscription open should not hang")
            .expect("subscription should open");
            let initial_event = timeout(operation_timeout(), subscription.next_event())
                .await
                .expect("subscription next_event should not hang")
                .expect("subscription should stay healthy")
                .expect("subscription should yield an event");
            assert!(
                matches!(
                    initial_event,
                    NodeSubscriptionEvent::ScreenDelta(delta) if delta.full_replace.is_some()
                ),
                "cycle {cycle} should receive an initial pane delta"
            );
            timeout(operation_timeout(), subscription.close())
                .await
                .expect("subscription close should not hang");

            timeout(operation_timeout(), server.shutdown())
                .await
                .expect("daemon shutdown should not hang")
                .expect("daemon should stop cleanly");

            let stale_result = timeout(operation_timeout(), node.list_sessions())
                .await
                .expect("stale list_sessions should not hang");
            assert!(stale_result.is_err(), "cycle {cycle} stale request should fail");
        }
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
        assert!(export_dir.join("NodeSubscriptionSpec.ts").exists());
        assert!(export_dir.join("NodeSubscriptionEvent.ts").exists());
        assert!(export_dir.join("NodeAttachedSession.ts").exists());
        let binding = std::fs::read_to_string(export_dir.join("NodeHandshakeInfo.ts"))
            .expect("handshake binding should be readable");
        assert!(binding.contains("NodeHandshakeInfo"));
    }

    fn cat_launch_request(title: &str) -> NodeCreateSessionRequest {
        let launch = echo_shell_launch_spec();
        NodeCreateSessionRequest {
            title: Some(title.to_string()),
            launch: Some(super::NodeShellLaunchSpec {
                program: launch.program,
                args: launch.args,
                cwd: launch.cwd.map(|cwd| cwd.display().to_string()),
            }),
        }
    }

    async fn wait_for_screen_line(
        node: &NodeHostClient,
        session_id: &str,
        pane_id: &str,
        needle: &str,
    ) -> super::NodeScreenSnapshot {
        for _ in 0..screen_wait_attempts() {
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

    async fn wait_for_interactive_screen(
        node: &NodeHostClient,
        session_id: &str,
        pane_id: &str,
        label: &str,
    ) -> super::NodeScreenSnapshot {
        let marker = format!("node-interactive-probe-{label}-{}", std::process::id());

        for attempt in 0..screen_wait_attempts() {
            if attempt % interactive_probe_interval() == 0 {
                timeout(
                    operation_timeout(),
                    node.dispatch_mux_command(
                        session_id,
                        &NodeMuxCommand::SendInput(NodeSendInputCommand {
                            pane_id: pane_id.to_string(),
                            data: submitted_input(&marker),
                        }),
                    ),
                )
                .await
                .expect("interactive probe send_input should not hang")
                .expect("interactive probe send_input should succeed");
            }

            let snapshot = node
                .screen_snapshot(session_id, pane_id)
                .await
                .expect("screen_snapshot should succeed");
            if snapshot.surface.lines.iter().any(|line| line.text.contains(&marker)) {
                return snapshot;
            }
            sleep(Duration::from_millis(100)).await;
        }

        panic!("screen never reached interactive probe marker: {marker}");
    }

    async fn next_topology_snapshot(
        subscription: &super::NodeSubscriptionHandle,
    ) -> Option<super::NodeTopologySnapshot> {
        for _ in 0..20 {
            match timeout(subscription_timeout(), subscription.next_event())
                .await
                .expect("subscription next_event should not hang")
                .expect("subscription should stay healthy")
            {
                Some(NodeSubscriptionEvent::TopologySnapshot(snapshot)) => return Some(snapshot),
                Some(NodeSubscriptionEvent::ScreenDelta(_)) => continue,
                None => return None,
            }
        }

        None
    }

    async fn wait_for_topology_state(
        node: &NodeHostClient,
        session_id: &str,
        predicate: impl Fn(&super::NodeTopologySnapshot) -> bool,
        label: &str,
    ) -> super::NodeTopologySnapshot {
        for _ in 0..screen_wait_attempts() {
            let snapshot =
                node.topology_snapshot(session_id).await.expect("topology_snapshot should succeed");
            if predicate(&snapshot) {
                return snapshot;
            }
            sleep(Duration::from_millis(100)).await;
        }

        panic!("topology never reached expected state: {label}");
    }

    async fn wait_for_subscription_line(
        subscription: &super::NodeSubscriptionHandle,
        needle: &str,
    ) -> Option<super::NodeScreenDelta> {
        for _ in 0..screen_wait_attempts() {
            match timeout(subscription_timeout(), subscription.next_event())
                .await
                .expect("subscription next_event should not hang")
                .expect("subscription should stay healthy")
            {
                Some(NodeSubscriptionEvent::ScreenDelta(delta))
                    if subscription_delta_contains(&delta, needle) =>
                {
                    return Some(delta);
                }
                Some(NodeSubscriptionEvent::ScreenDelta(_)) => continue,
                Some(NodeSubscriptionEvent::TopologySnapshot(_)) => continue,
                None => return None,
            }
        }

        None
    }

    async fn wait_for_subscription_close(subscription: &super::NodeSubscriptionHandle) -> bool {
        timeout(subscription_timeout(), async {
            for _ in 0..screen_wait_attempts() {
                match subscription
                    .next_event()
                    .await
                    .expect("subscription should stay healthy until closure")
                {
                    Some(_) => continue,
                    None => return true,
                }
            }

            false
        })
        .await
        .unwrap_or(false)
    }

    fn subscription_timeout() -> Duration {
        if cfg!(windows) { Duration::from_secs(60) } else { Duration::from_secs(5) }
    }

    fn operation_timeout() -> Duration {
        if cfg!(windows) { Duration::from_secs(60) } else { Duration::from_secs(5) }
    }

    fn extended_timeout() -> Duration {
        if cfg!(windows) { Duration::from_secs(90) } else { Duration::from_secs(10) }
    }

    fn zellij_operation_timeout() -> Duration {
        Duration::from_secs(90)
    }

    fn zellij_attempt_timeout() -> Duration {
        if cfg!(windows) { Duration::from_secs(240) } else { Duration::from_secs(90) }
    }

    fn screen_wait_attempts() -> usize {
        if cfg!(windows) { 900 } else { 50 }
    }

    fn interactive_probe_interval() -> usize {
        if cfg!(windows) { 20 } else { 10 }
    }

    fn submitted_input(text: &str) -> String {
        if cfg!(windows) { format!("{text}\n") } else { format!("{text}\r") }
    }

    fn spawn_daemon_with_retry(
        address: terminal_protocol::LocalSocketAddress,
    ) -> std::io::Result<terminal_daemon::LocalSocketServerHandle> {
        let attempts = if cfg!(windows) { 50 } else { 5 };
        let retryable_kinds = [
            std::io::ErrorKind::AlreadyExists,
            std::io::ErrorKind::PermissionDenied,
            std::io::ErrorKind::AddrInUse,
        ];
        let mut last_error = None;

        for attempt in 0..attempts {
            match spawn_local_socket_server(TerminalDaemon::new(daemon_state()), address.clone()) {
                Ok(server) => return Ok(server),
                Err(error) if retryable_kinds.contains(&error.kind()) && attempt + 1 < attempts => {
                    last_error = Some(error);
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(error) => return Err(error),
            }
        }

        Err(last_error.unwrap_or_else(|| std::io::Error::other("daemon never rebound on address")))
    }

    fn first_node_pane_id(root: &NodePaneTreeNode) -> Option<String> {
        match root {
            NodePaneTreeNode::Leaf { pane_id } => Some(pane_id.clone()),
            NodePaneTreeNode::Split(split) => {
                first_node_pane_id(&split.first).or_else(|| first_node_pane_id(&split.second))
            }
        }
    }

    fn subscription_delta_contains(delta: &super::NodeScreenDelta, needle: &str) -> bool {
        delta
            .patch
            .as_ref()
            .map(|patch| patch.line_updates.iter().any(|line| line.line.text.contains(needle)))
            .unwrap_or(false)
            || delta
                .full_replace
                .as_ref()
                .map(|surface| surface.lines.iter().any(|line| line.text.contains(needle)))
                .unwrap_or(false)
    }

    fn assert_zellij_delta_compatible_with_snapshot(
        snapshot: &super::NodeScreenSnapshot,
        delta: &super::NodeScreenDelta,
    ) {
        assert_eq!(delta.from_sequence, snapshot.sequence);
        assert!(
            delta.to_sequence >= snapshot.sequence,
            "zellij delta must not rewind sequence numbers"
        );
        if delta.to_sequence == snapshot.sequence {
            assert!(delta.patch.is_none());
            assert!(delta.full_replace.is_none());
        } else {
            assert!(delta.patch.is_none());
            assert!(delta.full_replace.is_some());
        }
    }

    async fn wait_for_discovered_zellij_session(
        node: &super::NodeHostClient,
        session_name: &str,
    ) -> super::NodeDiscoveredSession {
        for _ in 0..if cfg!(windows) { 200 } else { 100 } {
            let discovered =
                timeout(extended_timeout(), node.discover_sessions(NodeBackendKind::Zellij))
                    .await
                    .expect("discover_sessions should not hang")
                    .expect("discover_sessions should succeed");
            if let Some(candidate) = discovered
                .into_iter()
                .find(|session| session.title.as_deref() == Some(session_name))
            {
                return candidate;
            }
            sleep(Duration::from_millis(100)).await;
        }

        fallback_zellij_candidate(session_name)
    }

    fn fallback_zellij_candidate(session_name: &str) -> super::NodeDiscoveredSession {
        super::NodeDiscoveredSession {
            route: super::NodeSessionRoute {
                backend: NodeBackendKind::Zellij,
                authority: super::NodeRouteAuthority::ImportedForeign,
                external: Some(super::NodeExternalSessionRef {
                    namespace: "zellij_session".to_string(),
                    value: format!("session={session_name}"),
                }),
            },
            title: Some(session_name.to_string()),
        }
    }
}
