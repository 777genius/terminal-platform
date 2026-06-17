mod active_session_service;
mod capture_semantics;
mod catalog_service;
mod runtime;
mod saved_sessions_service;
mod service_api;
mod subscription_service;

use std::sync::Arc;

use terminal_persistence::SqliteSessionStore;

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
