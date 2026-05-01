use terminal_backend_api::{
    BackendCapabilities, BackendError, BackendSessionSummary, CreateSessionSpec, DiscoveredSession,
};
use terminal_domain::{BackendKind, SessionRoute};

use super::super::SessionService;

impl SessionService {
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

    #[must_use]
    pub fn available_backends(&self) -> Vec<BackendKind> {
        self.catalog_service().available_backends()
    }

    #[must_use]
    pub fn list_sessions(&self) -> Vec<BackendSessionSummary> {
        self.catalog_service().list_sessions()
    }

    #[must_use]
    pub fn session_count(&self) -> usize {
        self.catalog_service().session_count()
    }
}
