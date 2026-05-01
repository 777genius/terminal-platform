use terminal_backend_api::{
    BackendCapabilities, BackendError, BackendSessionSummary, BackendSubscription,
    CreateSessionSpec, DiscoveredSession, MuxCommand, MuxCommandResult, SubscriptionSpec,
};
use terminal_domain::{
    BackendKind, CURRENT_BINARY_VERSION, CURRENT_PROTOCOL_MAJOR, CURRENT_PROTOCOL_MINOR, PaneId,
    SessionId, SessionRoute,
};
use terminal_persistence::{
    CommandHistoryEntryRecord, PaneHistoryHydrationRecord, PrunedSavedSessions, SavedNativeSession,
    SavedSessionSummary, SqliteSessionStore,
};
use terminal_projection::{ScreenDelta, ScreenSnapshot, SessionHealthSnapshot, TopologySnapshot};

use crate::{
    BackendCatalog, RuntimeCapabilities, RuntimeHandshake, RuntimePhase, RuntimeProtocolVersion,
    TerminalRuntimeBuilder, sessions::SessionService,
};

pub struct TerminalRuntime {
    sessions: SessionService,
}

impl TerminalRuntime {
    #[must_use]
    pub fn builder() -> TerminalRuntimeBuilder {
        TerminalRuntimeBuilder::default()
    }

    #[must_use]
    pub fn new(backends: BackendCatalog) -> Self {
        Self::builder()
            .with_backends(backends)
            .with_default_persistence()
            .expect("default sqlite session store should open")
            .build()
            .expect("terminal runtime builder should have backends configured")
    }

    #[must_use]
    pub fn with_persistence(backends: BackendCatalog, persistence: SqliteSessionStore) -> Self {
        Self { sessions: SessionService::with_persistence(backends, persistence) }
    }

    #[must_use]
    pub fn handshake(&self) -> RuntimeHandshake {
        RuntimeHandshake {
            protocol_version: RuntimeProtocolVersion {
                major: CURRENT_PROTOCOL_MAJOR,
                minor: CURRENT_PROTOCOL_MINOR,
            },
            binary_version: CURRENT_BINARY_VERSION.to_string(),
            daemon_phase: RuntimePhase::Ready,
            capabilities: RuntimeCapabilities {
                request_reply: true,
                topology_subscriptions: true,
                pane_subscriptions: true,
                backend_discovery: true,
                backend_capability_queries: true,
                saved_sessions: true,
                session_restore: true,
                degraded_error_reasons: true,
                session_health: true,
            },
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

    pub fn list_saved_sessions(&self) -> Result<Vec<SavedSessionSummary>, BackendError> {
        self.sessions.list_saved_sessions()
    }

    pub fn saved_session(&self, session_id: SessionId) -> Result<SavedNativeSession, BackendError> {
        self.sessions.saved_session(session_id)
    }

    pub fn saved_session_v2_restore_plan(
        &self,
        session_id: SessionId,
    ) -> Result<Option<terminal_persistence::RestorePlan>, BackendError> {
        self.sessions.saved_session_v2_restore_plan(session_id)
    }

    pub fn delete_saved_session(&self, session_id: SessionId) -> Result<(), BackendError> {
        self.sessions.delete_saved_session(session_id)
    }

    pub fn prune_saved_sessions(
        &self,
        keep_latest: usize,
    ) -> Result<PrunedSavedSessions, BackendError> {
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

    pub async fn pane_history(
        &self,
        session_id: SessionId,
        pane_id: PaneId,
        from_event_seq: Option<i64>,
        max_segments: Option<i64>,
        max_bytes: Option<i64>,
    ) -> Result<PaneHistoryHydrationRecord, BackendError> {
        self.sessions
            .pane_history(session_id, pane_id, from_event_seq, max_segments, max_bytes)
            .await
    }

    pub async fn command_history(
        &self,
        session_id: Option<SessionId>,
        limit: Option<i64>,
    ) -> Result<Vec<CommandHistoryEntryRecord>, BackendError> {
        self.sessions.command_history(session_id, limit).await
    }

    pub async fn dispatch(
        &self,
        session_id: SessionId,
        command: MuxCommand,
    ) -> Result<MuxCommandResult, BackendError> {
        self.sessions.dispatch(session_id, command).await
    }

    pub fn session_health_snapshot(
        &self,
        session_id: SessionId,
    ) -> Result<SessionHealthSnapshot, BackendError> {
        self.sessions.session_health_snapshot(session_id)
    }

    pub async fn open_subscription(
        &self,
        session_id: SessionId,
        spec: SubscriptionSpec,
    ) -> Result<BackendSubscription, BackendError> {
        self.sessions.open_subscription(session_id, spec).await
    }
}
