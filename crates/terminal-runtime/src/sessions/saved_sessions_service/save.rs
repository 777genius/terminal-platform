use terminal_backend_api::{BackendError, MuxCommandResult};
use terminal_domain::{BackendKind, DegradedModeReason, SavedSessionManifest, SessionId};
use terminal_persistence::{SavedNativeSession, SqliteSessionStore};

use super::{
    super::runtime::{collect_pane_ids_from_topology, saved_session_title},
    SavedSessionsService,
};

impl SavedSessionsService<'_> {
    pub(in crate::sessions) async fn save_session(
        &self,
        session_id: SessionId,
    ) -> Result<MuxCommandResult, BackendError> {
        let descriptor =
            self.runtime.registry().get(session_id).ok_or_else(|| {
                BackendError::not_found(format!("unknown session {session_id:?}"))
            })?;
        if descriptor.route.backend != BackendKind::Native {
            return Err(BackendError::unsupported(
                "saved-session persistence currently supports native runtime sessions only; imported multiplexor sessions expose live control history but not saved-session restore guarantees",
                DegradedModeReason::UnsupportedByBackend,
            ));
        }

        let session = self.runtime.attach_session(session_id).await?;
        let topology = session.topology_snapshot().await?;
        let mut screens = Vec::new();
        for pane_id in collect_pane_ids_from_topology(&topology) {
            screens.push(session.screen_snapshot(pane_id).await?);
        }

        let snapshot = SavedNativeSession {
            session_id,
            route: descriptor.route,
            title: saved_session_title(descriptor.title, &topology),
            launch: descriptor.launch,
            manifest: SavedSessionManifest::current(),
            topology,
            screens,
            saved_at_ms: SqliteSessionStore::save_timestamp_ms().map_err(|error| {
                BackendError::internal(format!("failed to prepare save timestamp - {error}"))
            })?,
        };
        self.runtime.persistence().save_native_session_v2_snapshot(&snapshot).map_err(|error| {
            BackendError::internal(format!(
                "failed to persist native session v2 snapshot - {error}"
            ))
        })?;
        self.runtime.persistence().save_native_session(&snapshot).map_err(|error| {
            BackendError::internal(format!("failed to publish saved native session - {error}"))
        })?;

        Ok(MuxCommandResult { changed: false })
    }
}
