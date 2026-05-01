use terminal_backend_api::BackendError;
use terminal_domain::{PaneId, SessionId};
use terminal_projection::{ScreenDelta, ScreenSnapshot, TopologySnapshot};

use super::ActiveSessionService;

impl ActiveSessionService<'_> {
    pub(in crate::sessions) async fn topology_snapshot(
        &self,
        session_id: SessionId,
    ) -> Result<TopologySnapshot, BackendError> {
        let session = self.runtime.attach_session(session_id).await?;
        session.topology_snapshot().await
    }

    pub(in crate::sessions) async fn screen_snapshot(
        &self,
        session_id: SessionId,
        pane_id: PaneId,
    ) -> Result<ScreenSnapshot, BackendError> {
        let session = self.runtime.attach_session(session_id).await?;
        session.screen_snapshot(pane_id).await
    }

    pub(in crate::sessions) async fn screen_delta(
        &self,
        session_id: SessionId,
        pane_id: PaneId,
        from_sequence: u64,
    ) -> Result<ScreenDelta, BackendError> {
        let session = self.runtime.attach_session(session_id).await?;
        session.screen_delta(pane_id, from_sequence).await
    }
}
