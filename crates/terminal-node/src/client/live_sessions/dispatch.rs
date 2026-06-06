use terminal_protocol::ProtocolError;

use crate::{NodeHostClient, dto::*, ids::parse_session_id};

impl NodeHostClient {
    pub async fn dispatch_mux_command(
        &self,
        session_id: &str,
        command: &NodeMuxCommand,
    ) -> Result<NodeMuxCommandResult, ProtocolError> {
        let result =
            self.client.dispatch(parse_session_id(session_id)?, command.try_into()?).await?;
        Ok((&result).into())
    }
}
