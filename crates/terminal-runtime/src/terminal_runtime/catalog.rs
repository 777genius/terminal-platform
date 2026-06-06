use terminal_backend_api::{
    BackendCapabilities, BackendError, BackendSessionSummary, CreateSessionSpec, DiscoveredSession,
};
use terminal_domain::{BackendKind, SessionRoute};

use super::TerminalRuntime;

impl TerminalRuntime {
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
}
