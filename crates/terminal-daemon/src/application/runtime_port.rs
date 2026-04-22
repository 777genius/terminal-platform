use terminal_backend_api::{
    BackendCapabilities, BackendError, BackendSessionSummary, BackendSubscription,
    CreateSessionSpec, DiscoveredSession, MuxCommand, MuxCommandResult, SubscriptionSpec,
};
use terminal_domain::{BackendKind, PaneId, SessionId, SessionRoute};
use terminal_projection::{
    ScreenDelta, ScreenSnapshot, SessionHealthSnapshot, TopologySnapshot,
};
use terminal_protocol::Handshake;

use super::{RuntimePrunedSavedSessions, RuntimeSavedSessionRecord, RuntimeSavedSessionSummary};

pub trait TerminalDaemonCatalogPort {
    fn handshake(&self) -> Handshake;
    fn list_sessions(&self) -> Vec<BackendSessionSummary>;

    async fn create_session(
        &self,
        backend: BackendKind,
        spec: CreateSessionSpec,
    ) -> Result<BackendSessionSummary, BackendError>;
    async fn discover_sessions(
        &self,
        backend: BackendKind,
    ) -> Result<Vec<DiscoveredSession>, BackendError>;
    async fn backend_capabilities(
        &self,
        backend: BackendKind,
    ) -> Result<BackendCapabilities, BackendError>;
    async fn import_session(
        &self,
        route: SessionRoute,
        title: Option<String>,
    ) -> Result<BackendSessionSummary, BackendError>;
}

pub trait TerminalDaemonSavedSessionsPort {
    fn list_saved_sessions(&self) -> Result<Vec<RuntimeSavedSessionSummary>, BackendError>;
    fn saved_session(
        &self,
        session_id: SessionId,
    ) -> Result<RuntimeSavedSessionRecord, BackendError>;
    fn delete_saved_session(&self, session_id: SessionId) -> Result<(), BackendError>;
    fn prune_saved_sessions(
        &self,
        keep_latest: usize,
    ) -> Result<RuntimePrunedSavedSessions, BackendError>;

    async fn restore_saved_session(
        &self,
        session_id: SessionId,
    ) -> Result<BackendSessionSummary, BackendError>;
}

pub trait TerminalDaemonActiveSessionPort {
    fn session_health_snapshot(
        &self,
        session_id: SessionId,
    ) -> Result<SessionHealthSnapshot, BackendError>;
    async fn topology_snapshot(
        &self,
        session_id: SessionId,
    ) -> Result<TopologySnapshot, BackendError>;
    async fn screen_snapshot(
        &self,
        session_id: SessionId,
        pane_id: PaneId,
    ) -> Result<ScreenSnapshot, BackendError>;
    async fn screen_delta(
        &self,
        session_id: SessionId,
        pane_id: PaneId,
        from_sequence: u64,
    ) -> Result<ScreenDelta, BackendError>;
    async fn dispatch(
        &self,
        session_id: SessionId,
        command: MuxCommand,
    ) -> Result<MuxCommandResult, BackendError>;
}

pub trait TerminalDaemonSubscriptionPort {
    async fn open_subscription(
        &self,
        session_id: SessionId,
        spec: SubscriptionSpec,
    ) -> Result<BackendSubscription, BackendError>;
}
