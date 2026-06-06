use terminal_backend_api::BackendError;
use terminal_domain::{PaneId, SessionId};
use terminal_persistence::{
    CommandHistoryEntryRecord, PaneHistoryHydrationRecord, ScreenSnapshotEventInput,
};

use super::ActiveSessionService;
use crate::sessions::runtime::tab_id_for_pane;

impl ActiveSessionService<'_> {
    pub(in crate::sessions) async fn pane_history(
        &self,
        session_id: SessionId,
        pane_id: PaneId,
        from_event_seq: Option<i64>,
        max_segments: Option<i64>,
        max_bytes: Option<i64>,
    ) -> Result<PaneHistoryHydrationRecord, BackendError> {
        let history = self
            .hydrate_pane_history(session_id, pane_id, from_event_seq, max_segments, max_bytes)
            .await?;

        if should_refresh_live_rendered_snapshot(&history)
            && self.refresh_live_screen_snapshot(session_id, pane_id).await.is_ok()
        {
            return self
                .hydrate_pane_history(session_id, pane_id, from_event_seq, max_segments, max_bytes)
                .await;
        }

        Ok(history)
    }

    async fn hydrate_pane_history(
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

    async fn refresh_live_screen_snapshot(
        &self,
        session_id: SessionId,
        pane_id: PaneId,
    ) -> Result<(), BackendError> {
        let descriptor =
            self.runtime.registry().get(session_id).ok_or_else(|| {
                BackendError::not_found(format!("unknown session {session_id:?}"))
            })?;
        let session = self.runtime.attach_session(session_id).await?;
        let topology = session.topology_snapshot().await.ok();
        let tab_id = topology
            .as_ref()
            .and_then(|topology| tab_id_for_pane(topology, pane_id))
            .map(|tab_id| tab_id.0.to_string());
        let screen = session.screen_snapshot(pane_id).await?;

        let input = ScreenSnapshotEventInput {
            session_id: descriptor.session_id.0.to_string(),
            route: descriptor.route,
            title: descriptor.title,
            launch: descriptor.launch,
            tab_id,
            screen,
            buffer_kind: Some("normal".to_string()),
            capture_semantics: Some("rendered_plaintext_snapshot".to_string()),
        };
        let store = self.runtime.persistence().clone();
        tokio::task::spawn_blocking(move || store.record_v2_screen_snapshot(input))
            .await
            .map_err(|error| {
                BackendError::internal(format!("screen snapshot refresh task failed - {error}"))
            })?
            .map_err(|error| {
                BackendError::internal(format!("failed to refresh screen snapshot - {error}"))
            })?;
        Ok(())
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

fn should_refresh_live_rendered_snapshot(history: &PaneHistoryHydrationRecord) -> bool {
    history.segments.is_empty()
        && history.latest_screen_snapshot.is_some()
        && history.total_payload_bytes == 0
}
