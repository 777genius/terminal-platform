use terminal_backend_api::{
    BackendCapabilities, BackendError, BackendSessionSummary, BackendSubscription,
    CreateSessionSpec, DiscoveredSession, MuxCommand, MuxCommandResult, SubscriptionSpec,
};
use terminal_domain::{BackendKind, PaneId, SessionId, SessionRoute};
use terminal_projection::{
    ScreenDelta, ScreenSnapshot, SessionHealthSnapshot, TopologySnapshot,
};
use terminal_protocol::{DaemonCapabilities, DaemonPhase, Handshake, ProtocolVersion};
use terminal_runtime::{RuntimeHandshake, RuntimePhase, TerminalRuntime};

use crate::application::{
    RuntimePrunedSavedSessions, RuntimeSavedSessionRecord, RuntimeSavedSessionSummary,
    TerminalDaemonActiveSessionPort, TerminalDaemonCatalogPort, TerminalDaemonSavedSessionsPort,
    TerminalDaemonSubscriptionPort,
};

#[derive(Clone, Copy)]
pub struct TerminalRuntimeAdapter<'a> {
    runtime: &'a TerminalRuntime,
}

impl<'a> TerminalRuntimeAdapter<'a> {
    #[must_use]
    pub fn new(runtime: &'a TerminalRuntime) -> Self {
        Self { runtime }
    }
}

impl TerminalDaemonCatalogPort for TerminalRuntimeAdapter<'_> {
    fn handshake(&self) -> Handshake {
        map_runtime_handshake(self.runtime.handshake())
    }

    fn list_sessions(&self) -> Vec<BackendSessionSummary> {
        self.runtime.list_sessions()
    }

    async fn create_session(
        &self,
        backend: BackendKind,
        spec: CreateSessionSpec,
    ) -> Result<BackendSessionSummary, BackendError> {
        self.runtime.create_session(backend, spec).await
    }

    async fn discover_sessions(
        &self,
        backend: BackendKind,
    ) -> Result<Vec<DiscoveredSession>, BackendError> {
        self.runtime.discover_sessions(backend).await
    }

    async fn backend_capabilities(
        &self,
        backend: BackendKind,
    ) -> Result<BackendCapabilities, BackendError> {
        self.runtime.backend_capabilities(backend).await
    }

    async fn import_session(
        &self,
        route: SessionRoute,
        title: Option<String>,
    ) -> Result<BackendSessionSummary, BackendError> {
        self.runtime.import_session(route, title).await
    }
}

impl TerminalDaemonSavedSessionsPort for TerminalRuntimeAdapter<'_> {
    fn list_saved_sessions(&self) -> Result<Vec<RuntimeSavedSessionSummary>, BackendError> {
        self.runtime
            .list_saved_sessions()
            .map(|sessions| sessions.into_iter().map(map_saved_session_summary).collect())
    }

    fn saved_session(
        &self,
        session_id: SessionId,
    ) -> Result<RuntimeSavedSessionRecord, BackendError> {
        self.runtime.saved_session(session_id).map(map_saved_session_record)
    }

    fn delete_saved_session(&self, session_id: SessionId) -> Result<(), BackendError> {
        self.runtime.delete_saved_session(session_id)
    }

    fn prune_saved_sessions(
        &self,
        keep_latest: usize,
    ) -> Result<RuntimePrunedSavedSessions, BackendError> {
        self.runtime.prune_saved_sessions(keep_latest).map(|pruned| RuntimePrunedSavedSessions {
            deleted_count: pruned.deleted_count,
            kept_count: pruned.kept_count,
        })
    }

    async fn restore_saved_session(
        &self,
        session_id: SessionId,
    ) -> Result<BackendSessionSummary, BackendError> {
        self.runtime.restore_saved_session(session_id).await
    }
}

impl TerminalDaemonActiveSessionPort for TerminalRuntimeAdapter<'_> {
    fn session_health_snapshot(
        &self,
        session_id: SessionId,
    ) -> Result<SessionHealthSnapshot, BackendError> {
        self.runtime.session_health_snapshot(session_id)
    }

    async fn topology_snapshot(
        &self,
        session_id: SessionId,
    ) -> Result<TopologySnapshot, BackendError> {
        self.runtime.topology_snapshot(session_id).await
    }

    async fn screen_snapshot(
        &self,
        session_id: SessionId,
        pane_id: PaneId,
    ) -> Result<ScreenSnapshot, BackendError> {
        self.runtime.screen_snapshot(session_id, pane_id).await
    }

    async fn screen_delta(
        &self,
        session_id: SessionId,
        pane_id: PaneId,
        from_sequence: u64,
    ) -> Result<ScreenDelta, BackendError> {
        self.runtime.screen_delta(session_id, pane_id, from_sequence).await
    }

    async fn dispatch(
        &self,
        session_id: SessionId,
        command: MuxCommand,
    ) -> Result<MuxCommandResult, BackendError> {
        self.runtime.dispatch(session_id, command).await
    }
}

impl TerminalDaemonSubscriptionPort for TerminalRuntimeAdapter<'_> {
    async fn open_subscription(
        &self,
        session_id: SessionId,
        spec: SubscriptionSpec,
    ) -> Result<BackendSubscription, BackendError> {
        self.runtime.open_subscription(session_id, spec).await
    }
}

fn map_runtime_handshake(handshake: RuntimeHandshake) -> Handshake {
    Handshake {
        protocol_version: ProtocolVersion {
            major: handshake.protocol_version.major,
            minor: handshake.protocol_version.minor,
        },
        binary_version: handshake.binary_version,
        daemon_phase: match handshake.daemon_phase {
            RuntimePhase::Starting => DaemonPhase::Starting,
            RuntimePhase::Ready => DaemonPhase::Ready,
            RuntimePhase::Degraded => DaemonPhase::Degraded,
        },
        capabilities: DaemonCapabilities {
            request_reply: handshake.capabilities.request_reply,
            topology_subscriptions: handshake.capabilities.topology_subscriptions,
            pane_subscriptions: handshake.capabilities.pane_subscriptions,
            backend_discovery: handshake.capabilities.backend_discovery,
            backend_capability_queries: handshake.capabilities.backend_capability_queries,
            saved_sessions: handshake.capabilities.saved_sessions,
            session_restore: handshake.capabilities.session_restore,
            degraded_error_reasons: handshake.capabilities.degraded_error_reasons,
            session_health: handshake.capabilities.session_health,
        },
        available_backends: handshake.available_backends,
        session_scope: handshake.session_scope,
    }
}

fn map_saved_session_summary(
    session: terminal_persistence::SavedSessionSummary,
) -> RuntimeSavedSessionSummary {
    RuntimeSavedSessionSummary {
        session_id: session.session_id,
        route: session.route,
        title: session.title,
        saved_at_ms: session.saved_at_ms,
        manifest: session.manifest,
        has_launch: session.has_launch,
        tab_count: session.tab_count,
        pane_count: session.pane_count,
    }
}

fn map_saved_session_record(
    session: terminal_persistence::SavedNativeSession,
) -> RuntimeSavedSessionRecord {
    RuntimeSavedSessionRecord {
        session_id: session.session_id,
        route: session.route,
        title: session.title,
        launch: session.launch,
        manifest: session.manifest,
        topology: session.topology,
        screens: session.screens,
        saved_at_ms: session.saved_at_ms,
    }
}
