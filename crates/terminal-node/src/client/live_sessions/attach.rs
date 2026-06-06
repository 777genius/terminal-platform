use terminal_protocol::ProtocolError;

use crate::{
    NodeHostClient,
    dto::*,
    ids::{focused_pane_id, parse_session_id},
};

impl NodeHostClient {
    pub async fn attach_session(
        &self,
        session_id: &str,
    ) -> Result<NodeAttachedSession, ProtocolError> {
        let session_id = parse_session_id(session_id)?;
        let session = self
            .client
            .list_sessions()
            .await?
            .sessions
            .into_iter()
            .find(|session| session.session_id == session_id)
            .ok_or_else(|| {
                ProtocolError::new("session_not_found", format!("unknown session {session_id:?}"))
            })?;
        let health = self.client.session_health_snapshot(session_id).await?;
        let topology = self.client.topology_snapshot(session_id).await?;
        let focused_screen = match focused_pane_id(&topology) {
            Some(pane_id) => Some(self.client.screen_snapshot(session_id, pane_id).await?),
            None => None,
        };

        Ok(NodeAttachedSession {
            session: (&session).into(),
            health: (&health).into(),
            topology: (&topology).into(),
            focused_screen: focused_screen.as_ref().map(Into::into),
        })
    }
}
