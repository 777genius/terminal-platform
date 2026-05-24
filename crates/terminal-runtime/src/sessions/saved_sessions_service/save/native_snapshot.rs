use terminal_backend_api::BackendError;
use terminal_domain::{BackendKind, DegradedModeReason, SavedSessionManifest, SessionId};
use terminal_persistence::{SavedNativeSession, SqliteSessionStore};

use crate::sessions::runtime::{
    SessionRuntime, collect_pane_ids_from_topology, saved_session_title,
};

pub(super) struct SavedNativeSessionSnapshotCollector<'a> {
    runtime: SessionRuntime<'a>,
}

impl<'a> SavedNativeSessionSnapshotCollector<'a> {
    pub(super) fn new(runtime: SessionRuntime<'a>) -> Self {
        Self { runtime }
    }

    pub(super) async fn collect(
        self,
        session_id: SessionId,
    ) -> Result<SavedNativeSession, BackendError> {
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

        Ok(SavedNativeSession {
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
        })
    }
}
