use terminal_domain::{PaneId, SessionId};
use terminal_protocol::{
    CommandHistoryResponse, GetPaneHistoryRequest, ListCommandHistoryRequest, PaneHistoryResponse,
    ProtocolError, RequestPayload, ResponsePayload,
};

use super::LocalSocketDaemonClient;

impl LocalSocketDaemonClient {
    pub async fn pane_history(
        &self,
        session_id: SessionId,
        pane_id: PaneId,
        from_event_seq: Option<i64>,
        max_segments: Option<i64>,
        max_bytes: Option<i64>,
    ) -> Result<PaneHistoryResponse, ProtocolError> {
        let response = self
            .send_request(RequestPayload::GetPaneHistory(GetPaneHistoryRequest {
                session_id,
                pane_id,
                from_event_seq,
                max_segments,
                max_bytes,
            }))
            .await?;

        match response.payload {
            ResponsePayload::PaneHistory(history) => Ok(history),
            other => Err(ProtocolError::unexpected_payload("pane_history", &other)),
        }
    }

    pub async fn command_history(
        &self,
        session_id: Option<SessionId>,
        limit: Option<i64>,
    ) -> Result<CommandHistoryResponse, ProtocolError> {
        let response = self
            .send_request(RequestPayload::ListCommandHistory(ListCommandHistoryRequest {
                session_id,
                limit,
            }))
            .await?;

        match response.payload {
            ResponsePayload::CommandHistory(history) => Ok(history),
            other => Err(ProtocolError::unexpected_payload("command_history", &other)),
        }
    }
}
