use terminal_backend_api::BackendError;
use terminal_domain::{PaneId, SessionId};
use terminal_persistence::{CommandHistoryEntryRecord, PaneHistoryHydrationRecord};

use super::ActiveSessionService;

impl ActiveSessionService<'_> {
    pub(in crate::sessions) async fn pane_history(
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

    pub(in crate::sessions) async fn command_history(
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
}
