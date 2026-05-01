use terminal_domain::SessionId;
use terminal_protocol::{
    DeleteSavedSessionRequest, DeleteSavedSessionResponse, GetSavedSessionRequest,
    ListSavedSessionsResponse, ProtocolError, PruneSavedSessionsRequest,
    PruneSavedSessionsResponse, RequestPayload, ResponsePayload, RestoreSavedSessionRequest,
    RestoreSavedSessionResponse, SavedSessionResponse,
};

use super::LocalSocketDaemonClient;

impl LocalSocketDaemonClient {
    pub async fn list_saved_sessions(&self) -> Result<ListSavedSessionsResponse, ProtocolError> {
        let response = self.send_request(RequestPayload::ListSavedSessions).await?;

        match response.payload {
            ResponsePayload::ListSavedSessions(list) => Ok(list),
            other => Err(ProtocolError::unexpected_payload("list_saved_sessions", &other)),
        }
    }
    pub async fn saved_session(
        &self,
        session_id: SessionId,
    ) -> Result<SavedSessionResponse, ProtocolError> {
        let response = self
            .send_request(RequestPayload::GetSavedSession(GetSavedSessionRequest { session_id }))
            .await?;

        match response.payload {
            ResponsePayload::SavedSession(saved) => Ok(saved),
            other => Err(ProtocolError::unexpected_payload("saved_session", &other)),
        }
    }
    pub async fn delete_saved_session(
        &self,
        session_id: SessionId,
    ) -> Result<DeleteSavedSessionResponse, ProtocolError> {
        let response = self
            .send_request(RequestPayload::DeleteSavedSession(DeleteSavedSessionRequest {
                session_id,
            }))
            .await?;

        match response.payload {
            ResponsePayload::DeleteSavedSession(deleted) => Ok(deleted),
            other => Err(ProtocolError::unexpected_payload("delete_saved_session", &other)),
        }
    }

    pub async fn prune_saved_sessions(
        &self,
        keep_latest: usize,
    ) -> Result<PruneSavedSessionsResponse, ProtocolError> {
        let response = self
            .send_request(RequestPayload::PruneSavedSessions(PruneSavedSessionsRequest {
                keep_latest,
            }))
            .await?;

        match response.payload {
            ResponsePayload::PruneSavedSessions(pruned) => Ok(pruned),
            other => Err(ProtocolError::unexpected_payload("prune_saved_sessions", &other)),
        }
    }

    pub async fn restore_saved_session(
        &self,
        session_id: SessionId,
    ) -> Result<RestoreSavedSessionResponse, ProtocolError> {
        let response = self
            .send_request(RequestPayload::RestoreSavedSession(RestoreSavedSessionRequest {
                session_id,
            }))
            .await?;

        match response.payload {
            ResponsePayload::RestoreSavedSession(restored) => Ok(restored),
            other => Err(ProtocolError::unexpected_payload("restore_saved_session", &other)),
        }
    }
}
