use terminal_backend_api::BackendError;
use terminal_domain::SessionId;
use terminal_persistence::{
    PrunedSavedSessions, RestorePlan, SavedNativeSession,
    SavedSessionSummary as PersistedSavedSessionSummary,
};

use super::SavedSessionsService;

impl SavedSessionsService<'_> {
    pub(in crate::sessions) fn list_saved_sessions(
        &self,
    ) -> Result<Vec<PersistedSavedSessionSummary>, BackendError> {
        self.runtime.persistence().list_native_sessions().map_err(|error| {
            BackendError::internal(format!("failed to list saved native sessions - {error}"))
        })
    }

    pub(in crate::sessions) fn saved_session(
        &self,
        session_id: SessionId,
    ) -> Result<SavedNativeSession, BackendError> {
        self.runtime
            .persistence()
            .load_native_session(session_id)
            .map_err(|error| {
                BackendError::internal(format!("failed to load saved native session - {error}"))
            })?
            .ok_or_else(|| BackendError::not_found(format!("unknown saved session {session_id:?}")))
    }

    pub(in crate::sessions) fn saved_session_v2_restore_plan(
        &self,
        session_id: SessionId,
    ) -> Result<Option<RestorePlan>, BackendError> {
        self.runtime.persistence().native_session_v2_restore_plan(session_id).map_err(|error| {
            BackendError::internal(format!(
                "failed to load saved session v2 restore plan - {error}"
            ))
        })
    }

    pub(in crate::sessions) fn delete_saved_session(
        &self,
        session_id: SessionId,
    ) -> Result<(), BackendError> {
        let deleted =
            self.runtime.persistence().delete_native_session(session_id).map_err(|error| {
                BackendError::internal(format!("failed to delete saved native session - {error}"))
            })?;
        if !deleted {
            return Err(BackendError::not_found(format!("unknown saved session {session_id:?}")));
        }

        Ok(())
    }

    pub(in crate::sessions) fn prune_saved_sessions(
        &self,
        keep_latest: usize,
    ) -> Result<PrunedSavedSessions, BackendError> {
        self.runtime.persistence().prune_native_sessions(keep_latest).map_err(|error| {
            BackendError::internal(format!("failed to prune saved native sessions - {error}"))
        })
    }
}
