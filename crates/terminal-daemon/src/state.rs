use std::sync::Arc;

use terminal_application::{BackendCatalog, SessionService};
use terminal_backend_api::{
    BackendCapabilities, BackendError, BackendSessionSummary, BackendSubscription,
    CreateSessionSpec, DiscoveredSession, MuxBackendPort, MuxCommand, MuxCommandResult,
    SubscriptionSpec,
};
use terminal_backend_native::NativeBackend;
use terminal_backend_tmux::TmuxBackend;
use terminal_backend_zellij::ZellijBackend;
use terminal_domain::{BackendKind, PaneId, SessionId, SessionRoute};
use terminal_projection::{ScreenDelta, ScreenSnapshot, TopologySnapshot};
use terminal_protocol::{DaemonPhase, Handshake, ProtocolVersion};

pub struct TerminalDaemonState {
    sessions: SessionService,
}

impl Default for TerminalDaemonState {
    fn default() -> Self {
        Self::new(default_backend_catalog())
    }
}

impl TerminalDaemonState {
    #[must_use]
    pub fn new(backends: BackendCatalog) -> Self {
        Self { sessions: SessionService::new(backends) }
    }

    #[must_use]
    pub fn handshake(&self) -> Handshake {
        Handshake {
            protocol_version: ProtocolVersion { major: 0, minor: 1 },
            binary_version: "0.1.0-dev".to_string(),
            daemon_phase: DaemonPhase::Starting,
            capabilities: BackendCapabilities::default(),
            available_backends: vec![BackendKind::Native, BackendKind::Tmux, BackendKind::Zellij],
            session_scope: "current_user".to_string(),
        }
    }

    #[must_use]
    pub fn session_count(&self) -> usize {
        self.sessions.session_count()
    }

    #[must_use]
    pub fn list_sessions(&self) -> Vec<BackendSessionSummary> {
        self.sessions.list_sessions()
    }

    pub async fn discover_sessions(
        &self,
        backend: BackendKind,
    ) -> Result<Vec<DiscoveredSession>, BackendError> {
        self.sessions.discover_sessions(backend).await
    }

    pub async fn backend_capabilities(
        &self,
        backend: BackendKind,
    ) -> Result<BackendCapabilities, BackendError> {
        self.sessions.backend_capabilities(backend).await
    }

    pub async fn create_session(
        &self,
        backend: BackendKind,
        spec: CreateSessionSpec,
    ) -> Result<BackendSessionSummary, BackendError> {
        self.sessions.create_session(backend, spec).await
    }

    pub async fn import_session(
        &self,
        route: SessionRoute,
        title: Option<String>,
    ) -> Result<BackendSessionSummary, BackendError> {
        self.sessions.import_session(route, title).await
    }

    pub async fn topology_snapshot(
        &self,
        session_id: SessionId,
    ) -> Result<TopologySnapshot, BackendError> {
        self.sessions.topology_snapshot(session_id).await
    }

    pub async fn screen_snapshot(
        &self,
        session_id: SessionId,
        pane_id: PaneId,
    ) -> Result<ScreenSnapshot, BackendError> {
        self.sessions.screen_snapshot(session_id, pane_id).await
    }

    pub async fn screen_delta(
        &self,
        session_id: SessionId,
        pane_id: PaneId,
        from_sequence: u64,
    ) -> Result<ScreenDelta, BackendError> {
        self.sessions.screen_delta(session_id, pane_id, from_sequence).await
    }

    pub async fn dispatch(
        &self,
        session_id: SessionId,
        command: MuxCommand,
    ) -> Result<MuxCommandResult, BackendError> {
        self.sessions.dispatch(session_id, command).await
    }

    pub async fn open_subscription(
        &self,
        session_id: SessionId,
        spec: SubscriptionSpec,
    ) -> Result<BackendSubscription, BackendError> {
        self.sessions.open_subscription(session_id, spec).await
    }
}

fn default_backend_catalog() -> BackendCatalog {
    BackendCatalog::new([
        Arc::new(NativeBackend::default()) as Arc<dyn MuxBackendPort>,
        Arc::new(TmuxBackend::default()) as Arc<dyn MuxBackendPort>,
        Arc::new(ZellijBackend) as Arc<dyn MuxBackendPort>,
    ])
}

#[cfg(test)]
mod tests {
    use terminal_backend_api::{CreateSessionSpec, MuxCommand, NewTabSpec};
    use terminal_domain::BackendKind;

    use super::TerminalDaemonState;

    #[test]
    fn exposes_starting_handshake_with_known_backends() {
        let state = TerminalDaemonState::default();
        let handshake = state.handshake();

        assert_eq!(handshake.protocol_version.major, 0);
        assert_eq!(handshake.protocol_version.minor, 1);
        assert_eq!(handshake.available_backends.len(), 3);
        assert_eq!(state.session_count(), 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn creates_native_session_summary() {
        let state = TerminalDaemonState::default();
        let created = state
            .create_session(
                BackendKind::Native,
                CreateSessionSpec {
                    title: Some("shell".to_string()),
                    ..CreateSessionSpec::default()
                },
            )
            .await
            .expect("native session create should succeed");

        assert_eq!(created.route.backend, BackendKind::Native);
        assert_eq!(created.title.as_deref(), Some("shell"));
        assert_eq!(state.session_count(), 1);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn returns_dynamic_backend_capabilities() {
        let state = TerminalDaemonState::default();
        let native = state
            .backend_capabilities(BackendKind::Native)
            .await
            .expect("native capabilities should resolve");

        assert!(native.tiled_panes);
        assert!(native.split_resize);
        assert!(native.tab_create);
        assert!(native.tab_close);
        assert!(native.tab_focus);
        assert!(native.tab_rename);
        assert!(native.pane_split);
        assert!(native.pane_close);
        assert!(native.pane_focus);
        assert!(native.pane_input_write);
        assert!(native.layout_dump);
        assert!(native.layout_override);
        assert!(native.explicit_session_save);
        assert!(native.rendered_viewport_stream);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn returns_stub_topology_and_screen_for_native_session() {
        let state = TerminalDaemonState::default();
        let created = state
            .create_session(
                BackendKind::Native,
                CreateSessionSpec {
                    title: Some("shell".to_string()),
                    ..CreateSessionSpec::default()
                },
            )
            .await
            .expect("native session create should succeed");
        let topology = state
            .topology_snapshot(created.session_id)
            .await
            .expect("topology snapshot should succeed");
        let pane_id = topology.tabs[0].focused_pane.expect("focused pane should exist");
        let screen = state
            .screen_snapshot(created.session_id, pane_id)
            .await
            .expect("screen snapshot should succeed");

        assert_eq!(topology.session_id, created.session_id);
        assert_eq!(screen.pane_id, pane_id);
        assert!(!screen.surface.lines.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn returns_screen_delta_for_native_session() {
        let state = TerminalDaemonState::default();
        let created = state
            .create_session(
                BackendKind::Native,
                CreateSessionSpec {
                    title: Some("shell".to_string()),
                    ..CreateSessionSpec::default()
                },
            )
            .await
            .expect("native session create should succeed");
        let topology = state
            .topology_snapshot(created.session_id)
            .await
            .expect("topology snapshot should succeed");
        let pane_id = topology.tabs[0].focused_pane.expect("focused pane should exist");
        let snapshot = state
            .screen_snapshot(created.session_id, pane_id)
            .await
            .expect("screen snapshot should succeed");
        let delta = state
            .screen_delta(created.session_id, pane_id, snapshot.sequence)
            .await
            .expect("screen delta should succeed");

        assert_eq!(delta.pane_id, pane_id);
        assert_eq!(delta.from_sequence, snapshot.sequence);
        assert_eq!(delta.to_sequence, snapshot.sequence);
        assert_eq!(delta.rows, snapshot.rows);
        assert_eq!(delta.cols, snapshot.cols);
        assert!(delta.patch.is_none());
        assert!(delta.full_replace.is_none());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn dispatches_native_tab_mutations() {
        let state = TerminalDaemonState::default();
        let created = state
            .create_session(
                BackendKind::Native,
                CreateSessionSpec {
                    title: Some("shell".to_string()),
                    ..CreateSessionSpec::default()
                },
            )
            .await
            .expect("native session create should succeed");
        let result = state
            .dispatch(
                created.session_id,
                MuxCommand::NewTab(NewTabSpec { title: Some("logs".to_string()) }),
            )
            .await
            .expect("dispatch should succeed");
        let topology = state
            .topology_snapshot(created.session_id)
            .await
            .expect("topology snapshot should succeed");

        assert!(result.changed);
        assert_eq!(topology.tabs.len(), 2);
    }
}
