use terminal_backend_api::CreateSessionSpec;
use terminal_domain::{BackendKind, SessionRoute};
use terminal_protocol::{
    CreateSessionRequest, CreateSessionResponse, DiscoverSessionsRequest, DiscoverSessionsResponse,
    ImportSessionRequest, ImportSessionResponse, ListSessionsResponse, ProtocolError,
    RequestPayload, ResponsePayload,
};

use super::LocalSocketDaemonClient;

impl LocalSocketDaemonClient {
    pub async fn list_sessions(&self) -> Result<ListSessionsResponse, ProtocolError> {
        let response = self.send_request(RequestPayload::ListSessions).await?;

        match response.payload {
            ResponsePayload::ListSessions(list) => Ok(list),
            other => Err(ProtocolError::unexpected_payload("list_sessions", &other)),
        }
    }
    pub async fn discover_sessions(
        &self,
        backend: BackendKind,
    ) -> Result<DiscoverSessionsResponse, ProtocolError> {
        let response = self
            .send_request(RequestPayload::DiscoverSessions(DiscoverSessionsRequest { backend }))
            .await?;

        match response.payload {
            ResponsePayload::DiscoverSessions(discovered) => Ok(discovered),
            other => Err(ProtocolError::unexpected_payload("discover_sessions", &other)),
        }
    }
    pub async fn create_session(
        &self,
        backend: BackendKind,
        spec: CreateSessionSpec,
    ) -> Result<CreateSessionResponse, ProtocolError> {
        let response = self
            .send_request(RequestPayload::CreateSession(CreateSessionRequest { backend, spec }))
            .await?;

        match response.payload {
            ResponsePayload::CreateSession(created) => Ok(created),
            other => Err(ProtocolError::unexpected_payload("create_session", &other)),
        }
    }

    pub async fn import_session(
        &self,
        route: SessionRoute,
        title: Option<String>,
    ) -> Result<ImportSessionResponse, ProtocolError> {
        let response = self
            .send_request(RequestPayload::ImportSession(ImportSessionRequest { route, title }))
            .await?;

        match response.payload {
            ResponsePayload::ImportSession(imported) => Ok(imported),
            other => Err(ProtocolError::unexpected_payload("import_session", &other)),
        }
    }
}
