use terminal_backend_api::{
    BackendCapabilities, BackendError, BackendSessionSummary, BackendSubscription,
    CreateSessionSpec, DiscoveredSession, MuxCommand, MuxCommandResult, SubscriptionSpec,
};
use terminal_domain::{BackendKind, PaneId, SessionId, SessionRoute};
use terminal_projection::{ScreenDelta, ScreenSnapshot, TopologySnapshot};
use terminal_protocol::Handshake;

use crate::{
    TerminalDaemonState,
    application::{
        RuntimePrunedSavedSessions, RuntimeSavedSessionRecord, RuntimeSavedSessionSummary,
        TerminalDaemonRuntimePort,
    },
};

pub struct TerminalDaemonStateRuntimeAdapter<'a> {
    state: &'a TerminalDaemonState,
}

impl<'a> TerminalDaemonStateRuntimeAdapter<'a> {
    #[must_use]
    pub fn new(state: &'a TerminalDaemonState) -> Self {
        Self { state }
    }
}

impl TerminalDaemonRuntimePort for TerminalDaemonStateRuntimeAdapter<'_> {
    fn handshake(&self) -> Handshake {
        self.state.handshake()
    }

    fn list_sessions(&self) -> Vec<BackendSessionSummary> {
        self.state.list_sessions()
    }

    fn list_saved_sessions(&self) -> Result<Vec<RuntimeSavedSessionSummary>, BackendError> {
        self.state
            .list_saved_sessions()
            .map(|sessions| sessions.into_iter().map(map_saved_session_summary).collect())
    }

    fn saved_session(
        &self,
        session_id: SessionId,
    ) -> Result<RuntimeSavedSessionRecord, BackendError> {
        self.state.saved_session(session_id).map(map_saved_session_record)
    }

    fn delete_saved_session(&self, session_id: SessionId) -> Result<(), BackendError> {
        self.state.delete_saved_session(session_id)
    }

    fn prune_saved_sessions(
        &self,
        keep_latest: usize,
    ) -> Result<RuntimePrunedSavedSessions, BackendError> {
        self.state.prune_saved_sessions(keep_latest).map(|pruned| RuntimePrunedSavedSessions {
            deleted_count: pruned.deleted_count,
            kept_count: pruned.kept_count,
        })
    }

    async fn create_session(
        &self,
        backend: BackendKind,
        spec: CreateSessionSpec,
    ) -> Result<BackendSessionSummary, BackendError> {
        self.state.create_session(backend, spec).await
    }

    async fn discover_sessions(
        &self,
        backend: BackendKind,
    ) -> Result<Vec<DiscoveredSession>, BackendError> {
        self.state.discover_sessions(backend).await
    }

    async fn backend_capabilities(
        &self,
        backend: BackendKind,
    ) -> Result<BackendCapabilities, BackendError> {
        self.state.backend_capabilities(backend).await
    }

    async fn import_session(
        &self,
        route: SessionRoute,
        title: Option<String>,
    ) -> Result<BackendSessionSummary, BackendError> {
        self.state.import_session(route, title).await
    }

    async fn restore_saved_session(
        &self,
        session_id: SessionId,
    ) -> Result<BackendSessionSummary, BackendError> {
        self.state.restore_saved_session(session_id).await
    }

    async fn topology_snapshot(
        &self,
        session_id: SessionId,
    ) -> Result<TopologySnapshot, BackendError> {
        self.state.topology_snapshot(session_id).await
    }

    async fn screen_snapshot(
        &self,
        session_id: SessionId,
        pane_id: PaneId,
    ) -> Result<ScreenSnapshot, BackendError> {
        self.state.screen_snapshot(session_id, pane_id).await
    }

    async fn screen_delta(
        &self,
        session_id: SessionId,
        pane_id: PaneId,
        from_sequence: u64,
    ) -> Result<ScreenDelta, BackendError> {
        self.state.screen_delta(session_id, pane_id, from_sequence).await
    }

    async fn dispatch(
        &self,
        session_id: SessionId,
        command: MuxCommand,
    ) -> Result<MuxCommandResult, BackendError> {
        self.state.dispatch(session_id, command).await
    }

    async fn open_subscription(
        &self,
        session_id: SessionId,
        spec: SubscriptionSpec,
    ) -> Result<BackendSubscription, BackendError> {
        self.state.open_subscription(session_id, spec).await
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
