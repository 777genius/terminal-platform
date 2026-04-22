mod active_session_service;
mod catalog_service;
mod runtime;
mod saved_sessions_service;
mod subscription_service;

use std::sync::Arc;

use terminal_backend_api::{
    BackendCapabilities, BackendError, BackendSessionSummary, BackendSubscription,
    CreateSessionSpec, DiscoveredSession, MuxCommand, MuxCommandResult, SubscriptionSpec,
};
use terminal_domain::{BackendKind, PaneId, SessionId, SessionRoute};
use terminal_persistence::{
    PrunedSavedSessions, SavedNativeSession, SavedSessionSummary as PersistedSavedSessionSummary,
    SqliteSessionStore,
};
use terminal_projection::{
    ScreenDelta, ScreenSnapshot, SessionHealthSnapshot, TopologySnapshot,
};

use crate::{
    backend_catalog::BackendCatalog,
    registry::{InMemorySessionRegistry, SessionRegistry},
};

use self::{
    active_session_service::ActiveSessionService, catalog_service::SessionCatalogService,
    runtime::SessionRuntime, saved_sessions_service::SavedSessionsService,
    subscription_service::SessionSubscriptionService,
};

pub struct SessionService {
    backends: BackendCatalog,
    registry: Arc<InMemorySessionRegistry>,
    persistence: SqliteSessionStore,
}

impl SessionService {
    #[must_use]
    pub fn with_persistence(backends: BackendCatalog, persistence: SqliteSessionStore) -> Self {
        Self { backends, registry: Arc::new(InMemorySessionRegistry::default()), persistence }
    }

    pub async fn discover_sessions(
        &self,
        backend: BackendKind,
    ) -> Result<Vec<DiscoveredSession>, BackendError> {
        self.catalog_service().discover_sessions(backend).await
    }

    pub async fn backend_capabilities(
        &self,
        backend: BackendKind,
    ) -> Result<BackendCapabilities, BackendError> {
        self.catalog_service().backend_capabilities(backend).await
    }

    pub async fn create_session(
        &self,
        backend: BackendKind,
        spec: CreateSessionSpec,
    ) -> Result<BackendSessionSummary, BackendError> {
        self.catalog_service().create_session(backend, spec).await
    }

    pub async fn import_session(
        &self,
        route: SessionRoute,
        title: Option<String>,
    ) -> Result<BackendSessionSummary, BackendError> {
        self.catalog_service().import_session(route, title).await
    }

    pub fn list_saved_sessions(&self) -> Result<Vec<PersistedSavedSessionSummary>, BackendError> {
        self.saved_sessions_service().list_saved_sessions()
    }

    #[must_use]
    pub fn available_backends(&self) -> Vec<BackendKind> {
        self.catalog_service().available_backends()
    }

    pub fn saved_session(&self, session_id: SessionId) -> Result<SavedNativeSession, BackendError> {
        self.saved_sessions_service().saved_session(session_id)
    }

    pub fn delete_saved_session(&self, session_id: SessionId) -> Result<(), BackendError> {
        self.saved_sessions_service().delete_saved_session(session_id)
    }

    pub fn prune_saved_sessions(
        &self,
        keep_latest: usize,
    ) -> Result<PrunedSavedSessions, BackendError> {
        self.saved_sessions_service().prune_saved_sessions(keep_latest)
    }

    pub async fn restore_saved_session(
        &self,
        session_id: SessionId,
    ) -> Result<BackendSessionSummary, BackendError> {
        self.saved_sessions_service().restore_saved_session(session_id).await
    }

    #[must_use]
    pub fn list_sessions(&self) -> Vec<BackendSessionSummary> {
        self.catalog_service().list_sessions()
    }

    #[must_use]
    pub fn session_count(&self) -> usize {
        self.catalog_service().session_count()
    }

    pub async fn topology_snapshot(
        &self,
        session_id: SessionId,
    ) -> Result<TopologySnapshot, BackendError> {
        self.active_session_service().topology_snapshot(session_id).await
    }

    pub async fn screen_snapshot(
        &self,
        session_id: SessionId,
        pane_id: PaneId,
    ) -> Result<ScreenSnapshot, BackendError> {
        self.active_session_service().screen_snapshot(session_id, pane_id).await
    }

    pub async fn screen_delta(
        &self,
        session_id: SessionId,
        pane_id: PaneId,
        from_sequence: u64,
    ) -> Result<ScreenDelta, BackendError> {
        self.active_session_service().screen_delta(session_id, pane_id, from_sequence).await
    }

    pub async fn dispatch(
        &self,
        session_id: SessionId,
        command: MuxCommand,
    ) -> Result<MuxCommandResult, BackendError> {
        self.active_session_service().dispatch(session_id, command).await
    }

    pub fn session_health_snapshot(
        &self,
        session_id: SessionId,
    ) -> Result<SessionHealthSnapshot, BackendError> {
        self.active_session_service().session_health_snapshot(session_id)
    }

    pub async fn open_subscription(
        &self,
        session_id: SessionId,
        spec: SubscriptionSpec,
    ) -> Result<BackendSubscription, BackendError> {
        self.subscription_service().open_subscription(session_id, spec).await
    }

    fn runtime(&self) -> SessionRuntime<'_> {
        SessionRuntime::new(
            &self.backends,
            self.registry.clone() as Arc<dyn SessionRegistry>,
            &self.persistence,
        )
    }

    fn catalog_service(&self) -> SessionCatalogService<'_> {
        SessionCatalogService::new(self.runtime())
    }

    fn saved_sessions_service(&self) -> SavedSessionsService<'_> {
        SavedSessionsService::new(self.runtime())
    }

    fn active_session_service(&self) -> ActiveSessionService<'_> {
        ActiveSessionService::new(self.runtime())
    }

    fn subscription_service(&self) -> SessionSubscriptionService<'_> {
        SessionSubscriptionService::new(self.runtime())
    }
}
