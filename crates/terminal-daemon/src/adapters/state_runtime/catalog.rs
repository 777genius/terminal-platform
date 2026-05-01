use terminal_backend_api::{
    BackendCapabilities, BackendError, BackendSessionSummary, CreateSessionSpec, DiscoveredSession,
};
use terminal_domain::{BackendKind, SessionRoute};
use terminal_protocol::Handshake;

use crate::application::TerminalDaemonCatalogPort;

use super::{TerminalRuntimeAdapter, mappings::map_runtime_handshake};

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
