use terminal_backend_api::{BackendError, MuxCommand, MuxCommandResult};
use terminal_domain::{PaneId, SessionId};
use terminal_persistence::{
    CommandHistoryEntryRecord, PaneHistoryHydrationRecord, UiInputEventInput,
};
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

    pub(super) async fn pane_history(
        &self,
        session_id: SessionId,
        pane_id: PaneId,
        from_event_seq: Option<i64>,
        max_segments: Option<i64>,
        max_bytes: Option<i64>,
    ) -> Result<PaneHistoryHydrationRecord, BackendError> {
        let store = self.runtime.persistence().clone();
        tokio::task::spawn_blocking(move || {
            store.hydrate_v2_pane_history(
                &session_id.0.to_string(),
                &pane_id.0.to_string(),
                from_event_seq,
                max_segments,
                max_bytes,
            )
        })
        .await
        .map_err(|error| {
            BackendError::internal(format!("pane history read task failed - {error}"))
        })?
        .map_err(|error| {
            BackendError::internal(format!("failed to hydrate pane history - {error}"))
        })
    }

    pub(super) async fn command_history(
        &self,
        session_id: Option<SessionId>,
        limit: Option<i64>,
    ) -> Result<Vec<CommandHistoryEntryRecord>, BackendError> {
        let store = self.runtime.persistence().clone();
        tokio::task::spawn_blocking(move || {
            let session_id = session_id.map(|value| value.0.to_string());
            store.list_v2_command_history(session_id.as_deref(), limit.unwrap_or(100))
        })
        .await
        .map_err(|error| {
            BackendError::internal(format!("command history read task failed - {error}"))
        })?
        .map_err(|error| {
            BackendError::internal(format!("failed to list command history - {error}"))
        })
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
                source_event_id: spec.client_event_id.clone(),
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
                source_event_id: spec.client_event_id.clone(),
                rows: None,
                cols: None,
                shell_kind: None,
            }),
            _ => None,
        }
    }
}
