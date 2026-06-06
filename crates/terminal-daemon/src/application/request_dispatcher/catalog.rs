use terminal_protocol::{
    BackendCapabilitiesResponse, CreateSessionRequest, CreateSessionResponse,
    DiscoverSessionsRequest, DiscoverSessionsResponse, ImportSessionRequest, ImportSessionResponse,
    ListSessionsResponse, ProtocolError, ResponsePayload,
};

use crate::{
    adapters::map_backend_error,
    application::{
        TerminalDaemonActiveSessionPort, TerminalDaemonCatalogPort,
        TerminalDaemonSavedSessionsPort, TerminalDaemonSubscriptionPort,
    },
};

use super::TerminalDaemonRequestDispatcher;

impl<Catalog, SavedSessions, ActiveSessions, Subscriptions>
    TerminalDaemonRequestDispatcher<Catalog, SavedSessions, ActiveSessions, Subscriptions>
where
    Catalog: TerminalDaemonCatalogPort,
    SavedSessions: TerminalDaemonSavedSessionsPort,
    ActiveSessions: TerminalDaemonActiveSessionPort,
    Subscriptions: TerminalDaemonSubscriptionPort,
{
    pub(super) fn handshake_response(&self) -> ResponsePayload {
        ResponsePayload::Handshake(self.catalog.handshake())
    }

    pub(super) async fn create_session_response(
        &self,
        request: CreateSessionRequest,
    ) -> Result<ResponsePayload, ProtocolError> {
        let session = self
            .catalog
            .create_session(request.backend, request.spec)
            .await
            .map_err(map_backend_error)?;

        Ok(ResponsePayload::CreateSession(CreateSessionResponse { session }))
    }

    pub(super) fn list_sessions_response(&self) -> ResponsePayload {
        ResponsePayload::ListSessions(ListSessionsResponse {
            sessions: self.catalog.list_sessions(),
        })
    }

    pub(super) async fn discover_sessions_response(
        &self,
        request: DiscoverSessionsRequest,
    ) -> Result<ResponsePayload, ProtocolError> {
        Ok(ResponsePayload::DiscoverSessions(DiscoverSessionsResponse {
            sessions: self
                .catalog
                .discover_sessions(request.backend)
                .await
                .map_err(map_backend_error)?,
        }))
    }

    pub(super) async fn backend_capabilities_response(
        &self,
        request: terminal_protocol::GetBackendCapabilitiesRequest,
    ) -> Result<ResponsePayload, ProtocolError> {
        Ok(ResponsePayload::BackendCapabilities(BackendCapabilitiesResponse {
            backend: request.backend,
            capabilities: self
                .catalog
                .backend_capabilities(request.backend)
                .await
                .map_err(map_backend_error)?,
        }))
    }

    pub(super) async fn import_session_response(
        &self,
        request: ImportSessionRequest,
    ) -> Result<ResponsePayload, ProtocolError> {
        let session = self
            .catalog
            .import_session(request.route, request.title)
            .await
            .map_err(map_backend_error)?;

        Ok(ResponsePayload::ImportSession(ImportSessionResponse { session }))
    }
}
