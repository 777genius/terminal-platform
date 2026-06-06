use terminal_protocol::ProtocolError;

use crate::{
    NodeHostClient,
    dto::*,
    ids::{parse_pane_id, parse_session_id},
};

impl NodeHostClient {
    pub async fn pane_history(
        &self,
        session_id: &str,
        pane_id: &str,
        from_event_seq: Option<i64>,
        max_segments: Option<i64>,
        max_bytes: Option<i64>,
    ) -> Result<NodePaneHistory, ProtocolError> {
        let history = self
            .client
            .pane_history(
                parse_session_id(session_id)?,
                parse_pane_id(pane_id)?,
                from_event_seq,
                max_segments,
                max_bytes,
            )
            .await?;
        Ok((&history).into())
    }

    pub async fn command_history(
        &self,
        session_id: Option<&str>,
        limit: Option<i64>,
    ) -> Result<Vec<NodeCommandHistoryEntry>, ProtocolError> {
        let session_id = session_id.map(parse_session_id).transpose()?;
        let history = self.client.command_history(session_id, limit).await?;
        Ok(history.entries.iter().map(Into::into).collect())
    }
}
