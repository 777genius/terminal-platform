use terminal_protocol::ProtocolError;

use crate::{
    NodeHostClient,
    dto::*,
    ids::{parse_pane_id, parse_session_id},
};

impl NodeHostClient {
    pub async fn session_health_snapshot(
        &self,
        session_id: &str,
    ) -> Result<NodeSessionHealthSnapshot, ProtocolError> {
        let health = self.client.session_health_snapshot(parse_session_id(session_id)?).await?;
        Ok((&health).into())
    }

    pub async fn topology_snapshot(
        &self,
        session_id: &str,
    ) -> Result<NodeTopologySnapshot, ProtocolError> {
        let snapshot = self.client.topology_snapshot(parse_session_id(session_id)?).await?;
        Ok((&snapshot).into())
    }

    pub async fn screen_snapshot(
        &self,
        session_id: &str,
        pane_id: &str,
    ) -> Result<NodeScreenSnapshot, ProtocolError> {
        let snapshot = self
            .client
            .screen_snapshot(parse_session_id(session_id)?, parse_pane_id(pane_id)?)
            .await?;
        Ok((&snapshot).into())
    }

    pub async fn screen_delta(
        &self,
        session_id: &str,
        pane_id: &str,
        from_sequence: u64,
    ) -> Result<NodeScreenDelta, ProtocolError> {
        let delta = self
            .client
            .screen_delta(parse_session_id(session_id)?, parse_pane_id(pane_id)?, from_sequence)
            .await?;
        Ok((&delta).into())
    }
}
