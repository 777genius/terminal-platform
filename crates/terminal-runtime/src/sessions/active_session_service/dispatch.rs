use terminal_backend_api::{BackendError, MuxCommand, MuxCommandResult};
use terminal_domain::SessionId;

use super::{
    super::{runtime::command_updates_summary_title, saved_sessions_service::SavedSessionsService},
    ActiveSessionService, input_capture,
};

impl ActiveSessionService<'_> {
    pub(in crate::sessions) async fn dispatch(
        &self,
        session_id: SessionId,
        command: MuxCommand,
    ) -> Result<MuxCommandResult, BackendError> {
        if matches!(command, MuxCommand::SaveSession) {
            return SavedSessionsService::new(self.runtime.clone()).save_session(session_id).await;
        }

        let input_capture = input_capture::v2_input_capture(&self.runtime, session_id, &command);
        let session = self.runtime.attach_session(session_id).await?;
        let refresh_summary_title = command_updates_summary_title(&command);
        let result = session.dispatch(command).await?;

        if let Some(input_capture) = input_capture {
            let store = self.runtime.persistence().clone();
            let _ =
                tokio::task::spawn_blocking(move || store.record_v2_ui_input(input_capture)).await;
        }

        if result.changed && refresh_summary_title {
            self.runtime.refresh_session_summary_title(session_id, &*session).await;
        }

        Ok(result)
    }
}
