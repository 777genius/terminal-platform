use terminal_backend_api::{BackendError, MuxCommand, MuxCommandResult};
use terminal_domain::{PaneId, SessionId};
use terminal_persistence::UiInputEventInput;
use terminal_projection::{ScreenDelta, ScreenSnapshot, SessionHealthSnapshot, TopologySnapshot};

use super::{
    runtime::{SessionRuntime, command_updates_summary_title},
    saved_sessions_service::SavedSessionsService,
};

#[derive(Clone)]
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
            return SavedSessionsService::new(self.runtime.clone()).save_session(session_id).await;
        }
        let input_capture = self.v2_input_capture(session_id, &command);
        let session = self.runtime.attach_session(session_id).await?;
        let refresh_summary_title = command_updates_summary_title(&command);
        let result = session.dispatch(command).await?;
        if let Some(input_capture) = input_capture {
            let store = self.runtime.persistence().clone();
            let _ = tokio::task::spawn_blocking(move || {
                let _ = store.record_v2_ui_input(input_capture);
            });
        }
        if result.changed && refresh_summary_title {
            self.runtime.refresh_session_summary_title(session_id, &*session).await;
        }
        Ok(result)
    }

    pub(super) fn session_health_snapshot(
        &self,
        session_id: SessionId,
    ) -> Result<SessionHealthSnapshot, BackendError> {
        self.runtime.session_health_snapshot(session_id)
    }

    fn v2_input_capture(
        &self,
        session_id: SessionId,
        command: &MuxCommand,
    ) -> Option<UiInputEventInput> {
        let descriptor = self.runtime.registry().get(session_id)?;
        match command {
            MuxCommand::SendInput(spec) => Some(UiInputEventInput {
                session_id: session_id.0.to_string(),
                route: descriptor.route,
                title: descriptor.title,
                launch: descriptor.launch,
                pane_id: spec.pane_id.0.to_string(),
                data: spec.data.clone(),
                is_paste: false,
                rows: None,
                cols: None,
                shell_kind: None,
            }),
            MuxCommand::SendPaste(spec) => Some(UiInputEventInput {
                session_id: session_id.0.to_string(),
                route: descriptor.route,
                title: descriptor.title,
                launch: descriptor.launch,
                pane_id: spec.pane_id.0.to_string(),
                data: spec.data.clone(),
                is_paste: true,
                rows: None,
                cols: None,
                shell_kind: None,
            }),
            _ => None,
        }
    }
}
