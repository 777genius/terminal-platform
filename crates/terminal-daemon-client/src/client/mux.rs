use terminal_backend_api::{MuxCommand, MuxCommandResult};
use terminal_domain::SessionId;
use terminal_protocol::{
    DispatchMuxCommandRequest, ProtocolError, RequestPayload, ResponsePayload,
};

use super::LocalSocketDaemonClient;

impl LocalSocketDaemonClient {
    pub async fn dispatch(
        &self,
        session_id: SessionId,
        command: MuxCommand,
    ) -> Result<MuxCommandResult, ProtocolError> {
        let response = self
            .send_request(RequestPayload::DispatchMuxCommand(DispatchMuxCommandRequest {
                session_id,
                command,
            }))
            .await?;

        match response.payload {
            ResponsePayload::DispatchMuxCommand(result) => Ok(result),
            other => Err(ProtocolError::unexpected_payload("dispatch_mux_command", &other)),
        }
    }
}
