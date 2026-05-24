use terminal_backend_api::BackendError;
use terminal_persistence::{RestorePlan, SavedNativeSession, SqliteSessionStore};

pub(super) struct SavedSessionSaveOrchestrator<'a> {
    persistence: &'a SqliteSessionStore,
}

pub(super) struct SavedSessionSaveOutcome {
    #[allow(dead_code)]
    pub(super) restore_plan: RestorePlan,
}

impl<'a> SavedSessionSaveOrchestrator<'a> {
    pub(super) fn new(persistence: &'a SqliteSessionStore) -> Self {
        Self { persistence }
    }

    pub(super) fn save_native(
        &self,
        snapshot: SavedNativeSession,
    ) -> Result<SavedSessionSaveOutcome, BackendError> {
        let restore_plan = self.persist_v2_evidence(&snapshot)?;
        self.publish_legacy_snapshot(&snapshot)?;

        Ok(SavedSessionSaveOutcome { restore_plan })
    }

    fn persist_v2_evidence(
        &self,
        snapshot: &SavedNativeSession,
    ) -> Result<RestorePlan, BackendError> {
        self.persistence.save_native_session_v2_snapshot(snapshot).map_err(|error| {
            BackendError::internal(format!(
                "failed to persist native session v2 snapshot - {error}"
            ))
        })
    }

    fn publish_legacy_snapshot(&self, snapshot: &SavedNativeSession) -> Result<(), BackendError> {
        self.persistence.save_native_session(snapshot).map_err(|error| {
            BackendError::internal(format!("failed to publish saved native session - {error}"))
        })
    }
}
