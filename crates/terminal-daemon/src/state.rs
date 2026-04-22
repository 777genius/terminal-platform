use terminal_application::{BackendCatalog, SessionService};
use terminal_backend_api::{
    BackendCapabilities, BackendError, BackendSessionSummary, BackendSubscription,
    CreateSessionSpec, DiscoveredSession, MuxCommand, MuxCommandResult, SubscriptionSpec,
};
use terminal_domain::{
    BackendKind, CURRENT_BINARY_VERSION, CURRENT_PROTOCOL_MAJOR, CURRENT_PROTOCOL_MINOR, PaneId,
    SessionId, SessionRoute,
};
use terminal_persistence::{
    PrunedSavedSessions as PersistedPrunedSavedSessions, SavedNativeSession,
    SavedSessionSummary as PersistedSavedSessionSummary, SqliteSessionStore,
};
use terminal_projection::{ScreenDelta, ScreenSnapshot, TopologySnapshot};
use terminal_protocol::{DaemonCapabilities, DaemonPhase, Handshake, ProtocolVersion};
use thiserror::Error;

use crate::backend_registry::TerminalDaemonBackendRegistry;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerminalDaemonBackendConfig {
    pub native: bool,
    pub tmux: bool,
    pub zellij: bool,
}

impl TerminalDaemonBackendConfig {
    #[must_use]
    pub fn compiled_defaults() -> Self {
        Self {
            native: cfg!(feature = "native-backend"),
            tmux: cfg!(feature = "tmux-backend"),
            zellij: cfg!(feature = "zellij-backend"),
        }
    }

    #[must_use]
    pub const fn none() -> Self {
        Self { native: false, tmux: false, zellij: false }
    }

    #[must_use]
    pub const fn enable(mut self, backend: BackendKind, enabled: bool) -> Self {
        match backend {
            BackendKind::Native => self.native = enabled,
            BackendKind::Tmux => self.tmux = enabled,
            BackendKind::Zellij => self.zellij = enabled,
        }
        self
    }

    #[must_use]
    pub const fn is_enabled(&self, backend: BackendKind) -> bool {
        match backend {
            BackendKind::Native => self.native,
            BackendKind::Tmux => self.tmux,
            BackendKind::Zellij => self.zellij,
        }
    }

    #[must_use]
    pub fn enabled_backends(&self) -> Vec<BackendKind> {
        [BackendKind::Native, BackendKind::Tmux, BackendKind::Zellij]
            .into_iter()
            .filter(|backend| self.is_enabled(*backend))
            .collect()
    }
}

impl Default for TerminalDaemonBackendConfig {
    fn default() -> Self {
        Self::compiled_defaults()
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TerminalDaemonStateBuildError {
    #[error("terminal-daemon backend config enables no backends")]
    NoBackendsEnabled,
    #[error(
        "terminal-daemon backend {backend:?} was requested but is not compiled in. Compiled backends - {compiled_backends:?}"
    )]
    BackendNotCompiled { backend: BackendKind, compiled_backends: Vec<BackendKind> },
}

#[derive(Debug)]
pub struct TerminalDaemonStateBuilder {
    backend_config: TerminalDaemonBackendConfig,
    persistence: Option<SqliteSessionStore>,
    backend_registry: TerminalDaemonBackendRegistry,
}

impl Default for TerminalDaemonStateBuilder {
    fn default() -> Self {
        Self {
            backend_config: TerminalDaemonBackendConfig::default(),
            persistence: None,
            backend_registry: TerminalDaemonBackendRegistry::compiled_default(),
        }
    }
}

pub struct TerminalDaemonState {
    sessions: SessionService,
}

impl Default for TerminalDaemonState {
    fn default() -> Self {
        Self::builder()
            .build()
            .expect("compiled default terminal-daemon backend catalog should build")
    }
}

impl TerminalDaemonState {
    #[must_use]
    pub fn builder() -> TerminalDaemonStateBuilder {
        TerminalDaemonStateBuilder::default()
    }

    #[must_use]
    pub fn compiled_backends() -> Vec<BackendKind> {
        TerminalDaemonBackendRegistry::compiled_default().compiled_backends()
    }

    #[must_use]
    pub fn new(backends: BackendCatalog) -> Self {
        Self { sessions: SessionService::new(backends) }
    }

    pub fn with_backend_config(
        backend_config: TerminalDaemonBackendConfig,
    ) -> Result<Self, TerminalDaemonStateBuildError> {
        Self::builder().backend_config(backend_config).build()
    }

    pub fn with_backend_config_and_persistence(
        backend_config: TerminalDaemonBackendConfig,
        persistence: SqliteSessionStore,
    ) -> Result<Self, TerminalDaemonStateBuildError> {
        Self::builder().backend_config(backend_config).persistence(persistence).build()
    }

    #[must_use]
    pub fn with_default_persistence(persistence: SqliteSessionStore) -> Self {
        Self::with_backend_config_and_persistence(
            TerminalDaemonBackendConfig::default(),
            persistence,
        )
        .expect("compiled default terminal-daemon backend catalog should build with persistence")
    }

    #[must_use]
    pub fn handshake(&self) -> Handshake {
        Handshake {
            protocol_version: ProtocolVersion {
                major: CURRENT_PROTOCOL_MAJOR,
                minor: CURRENT_PROTOCOL_MINOR,
            },
            binary_version: CURRENT_BINARY_VERSION.to_string(),
            daemon_phase: DaemonPhase::Ready,
            capabilities: daemon_capabilities(),
            available_backends: self.sessions.available_backends(),
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

    pub fn list_saved_sessions(&self) -> Result<Vec<PersistedSavedSessionSummary>, BackendError> {
        self.sessions.list_saved_sessions()
    }

    pub fn saved_session(&self, session_id: SessionId) -> Result<SavedNativeSession, BackendError> {
        self.sessions.saved_session(session_id)
    }

    pub fn delete_saved_session(&self, session_id: SessionId) -> Result<(), BackendError> {
        self.sessions.delete_saved_session(session_id)
    }

    pub fn prune_saved_sessions(
        &self,
        keep_latest: usize,
    ) -> Result<PersistedPrunedSavedSessions, BackendError> {
        self.sessions.prune_saved_sessions(keep_latest)
    }

    pub async fn restore_saved_session(
        &self,
        session_id: SessionId,
    ) -> Result<BackendSessionSummary, BackendError> {
        self.sessions.restore_saved_session(session_id).await
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

impl TerminalDaemonStateBuilder {
    #[must_use]
    pub fn backend_config(mut self, backend_config: TerminalDaemonBackendConfig) -> Self {
        self.backend_config = backend_config;
        self
    }

    #[must_use]
    pub fn enable_backend(mut self, backend: BackendKind, enabled: bool) -> Self {
        self.backend_config = self.backend_config.enable(backend, enabled);
        self
    }

    #[must_use]
    pub fn persistence(mut self, persistence: SqliteSessionStore) -> Self {
        self.persistence = Some(persistence);
        self
    }

    #[must_use]
    pub fn backend_registry(mut self, backend_registry: TerminalDaemonBackendRegistry) -> Self {
        self.backend_registry = backend_registry;
        self
    }

    pub fn build(self) -> Result<TerminalDaemonState, TerminalDaemonStateBuildError> {
        let backends = self.backend_registry.build_catalog(self.backend_config)?;
        let sessions = match self.persistence {
            Some(persistence) => SessionService::with_persistence(backends, persistence),
            None => SessionService::new(backends),
        };
        Ok(TerminalDaemonState { sessions })
    }
}

fn daemon_capabilities() -> DaemonCapabilities {
    DaemonCapabilities {
        request_reply: true,
        topology_subscriptions: true,
        pane_subscriptions: true,
        backend_discovery: true,
        backend_capability_queries: true,
        saved_sessions: true,
        session_restore: true,
        degraded_error_reasons: true,
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "native-backend")]
    use std::sync::Arc;

    #[cfg(feature = "native-backend")]
    use terminal_application::BackendCatalog;
    #[cfg(feature = "native-backend")]
    use terminal_backend_api::{CreateSessionSpec, MuxBackendPort, MuxCommand, NewTabSpec};
    #[cfg(feature = "native-backend")]
    use terminal_backend_native::NativeBackend;
    use terminal_domain::BackendKind;
    #[cfg(feature = "native-backend")]
    use terminal_protocol::DaemonPhase;

    use super::{TerminalDaemonBackendConfig, TerminalDaemonState, TerminalDaemonStateBuildError};

    #[cfg(feature = "native-backend")]
    #[test]
    fn exposes_ready_handshake_with_configured_backends() {
        let state = TerminalDaemonState::new(BackendCatalog::new([
            Arc::new(NativeBackend::default()) as Arc<dyn MuxBackendPort>,
        ]));
        let handshake = state.handshake();

        assert_eq!(handshake.protocol_version.major, 0);
        assert_eq!(handshake.protocol_version.minor, 1);
        assert_eq!(handshake.daemon_phase, DaemonPhase::Ready);
        assert_eq!(handshake.available_backends, vec![BackendKind::Native]);
        assert!(handshake.capabilities.request_reply);
        assert!(handshake.capabilities.topology_subscriptions);
        assert!(handshake.capabilities.pane_subscriptions);
        assert!(handshake.capabilities.backend_discovery);
        assert!(handshake.capabilities.backend_capability_queries);
        assert!(handshake.capabilities.saved_sessions);
        assert!(handshake.capabilities.session_restore);
        assert!(handshake.capabilities.degraded_error_reasons);
        assert_eq!(state.session_count(), 0);
    }

    #[test]
    fn default_handshake_tracks_compiled_backends() {
        let state = TerminalDaemonState::default();
        let handshake = state.handshake();

        assert_eq!(handshake.available_backends, TerminalDaemonState::compiled_backends());
    }

    #[test]
    fn rejects_empty_backend_config() {
        let error = TerminalDaemonState::with_backend_config(TerminalDaemonBackendConfig::none())
            .err()
            .expect("empty backend config should fail");

        assert_eq!(error, TerminalDaemonStateBuildError::NoBackendsEnabled);
    }

    #[cfg(feature = "native-backend")]
    #[test]
    fn config_disables_compiled_backends() {
        let state = TerminalDaemonState::with_backend_config(
            TerminalDaemonBackendConfig::default()
                .enable(BackendKind::Tmux, false)
                .enable(BackendKind::Zellij, false),
        )
        .expect("native-only backend config should build");
        let handshake = state.handshake();

        assert_eq!(handshake.available_backends, vec![BackendKind::Native]);
    }

    #[cfg(not(feature = "native-backend"))]
    #[test]
    fn rejects_requesting_uncompiled_native_backend() {
        let error = TerminalDaemonState::builder()
            .enable_backend(BackendKind::Native, true)
            .build()
            .err()
            .expect("requesting uncompiled native backend should fail");

        assert_eq!(
            error,
            TerminalDaemonStateBuildError::BackendNotCompiled {
                backend: BackendKind::Native,
                compiled_backends: TerminalDaemonState::compiled_backends(),
            }
        );
    }

    #[cfg(not(feature = "tmux-backend"))]
    #[test]
    fn rejects_requesting_uncompiled_tmux_backend() {
        let error = TerminalDaemonState::builder()
            .enable_backend(BackendKind::Tmux, true)
            .build()
            .err()
            .expect("requesting uncompiled tmux backend should fail");

        assert_eq!(
            error,
            TerminalDaemonStateBuildError::BackendNotCompiled {
                backend: BackendKind::Tmux,
                compiled_backends: TerminalDaemonState::compiled_backends(),
            }
        );
    }

    #[cfg(not(feature = "zellij-backend"))]
    #[test]
    fn rejects_requesting_uncompiled_zellij_backend() {
        let error = TerminalDaemonState::builder()
            .enable_backend(BackendKind::Zellij, true)
            .build()
            .err()
            .expect("requesting uncompiled zellij backend should fail");

        assert_eq!(
            error,
            TerminalDaemonStateBuildError::BackendNotCompiled {
                backend: BackendKind::Zellij,
                compiled_backends: TerminalDaemonState::compiled_backends(),
            }
        );
    }

    #[cfg(feature = "native-backend")]
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

    #[cfg(feature = "native-backend")]
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
        assert!(native.explicit_session_restore);
        assert!(native.rendered_viewport_stream);
    }

    #[cfg(feature = "native-backend")]
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

    #[cfg(feature = "native-backend")]
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

    #[cfg(feature = "native-backend")]
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
