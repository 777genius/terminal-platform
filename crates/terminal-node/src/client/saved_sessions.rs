use terminal_protocol::ProtocolError;

use super::NodeHostClient;
use crate::{dto::*, ids::parse_session_id};

impl NodeHostClient {
    pub async fn list_saved_sessions(&self) -> Result<Vec<NodeSavedSessionSummary>, ProtocolError> {
        let listed = self.client.list_saved_sessions().await?;
        Ok(listed.sessions.iter().map(Into::into).collect())
    }

    pub async fn saved_session(
        &self,
        session_id: &str,
    ) -> Result<NodeSavedSessionRecord, ProtocolError> {
        let saved = self.client.saved_session(parse_session_id(session_id)?).await?;
        Ok((&saved.session).into())
    }

    pub async fn delete_saved_session(
        &self,
        session_id: &str,
    ) -> Result<NodeDeleteSavedSessionResult, ProtocolError> {
        let deleted = self.client.delete_saved_session(parse_session_id(session_id)?).await?;
        Ok((&deleted).into())
    }

    pub async fn prune_saved_sessions(
        &self,
        keep_latest: usize,
    ) -> Result<NodePruneSavedSessionsResult, ProtocolError> {
        let pruned = self.client.prune_saved_sessions(keep_latest).await?;
        Ok((&pruned).into())
    }

    pub async fn restore_saved_session(
        &self,
        session_id: &str,
    ) -> Result<NodeRestoredSession, ProtocolError> {
        let restored = self.client.restore_saved_session(parse_session_id(session_id)?).await?;
        Ok((&restored).into())
    }
}
