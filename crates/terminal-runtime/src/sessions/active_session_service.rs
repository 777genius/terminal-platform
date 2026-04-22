use terminal_backend_api::{BackendError, MuxCommand, MuxCommandResult};
use terminal_domain::{PaneId, SessionId};
use terminal_projection::{ScreenDelta, ScreenSnapshot, TopologySnapshot};

use super::{
    runtime::{SessionRuntime, command_updates_summary_title},
    saved_sessions_service::SavedSessionsService,
};

#[derive(Clone, Copy)]
pub(super) struct ActiveSessionService<'a> {
    runtime: SessionRuntime<'a>,
}

impl<'a> ActiveSessionService<'a> {
    pub(super) fn new(runtime: SessionRuntime<'a>) -> Self {
        Self { runtime }
    }

    pub(super) async fn topology_snapshot(
        &self,
        session_id: SessionId,
    ) -> Result<TopologySnapshot, BackendError> {
        let session = self.runtime.attach_session(session_id).await?;
        session.topology_snapshot().await
    }

    pub(super) async fn screen_snapshot(
        &self,
        session_id: SessionId,
        pane_id: PaneId,
    ) -> Result<ScreenSnapshot, BackendError> {
        let session = self.runtime.attach_session(session_id).await?;
        session.screen_snapshot(pane_id).await
    }

    pub(super) async fn screen_delta(
        &self,
        session_id: SessionId,
        pane_id: PaneId,
        from_sequence: u64,
    ) -> Result<ScreenDelta, BackendError> {
        let session = self.runtime.attach_session(session_id).await?;
        session.screen_delta(pane_id, from_sequence).await
    }

    pub(super) async fn dispatch(
        &self,
        session_id: SessionId,
        command: MuxCommand,
    ) -> Result<MuxCommandResult, BackendError> {
        if matches!(command, MuxCommand::SaveSession) {
            return SavedSessionsService::new(self.runtime).save_session(session_id).await;
        }
        let session = self.runtime.attach_session(session_id).await?;
        let refresh_summary_title = command_updates_summary_title(&command);
        let result = session.dispatch(command).await?;
        if result.changed && refresh_summary_title {
            self.runtime.refresh_session_summary_title(session_id, &*session).await;
        }
        Ok(result)
    }
}
