use terminal_backend_api::{BackendError, BackendSessionSummary};
use terminal_domain::SessionId;
use terminal_persistence::{
    PrunedSavedSessions, RestorePlan, SavedNativeSession,
    SavedSessionSummary as PersistedSavedSessionSummary,
};

use super::super::SessionService;

impl SessionService {
    pub fn list_saved_sessions(&self) -> Result<Vec<PersistedSavedSessionSummary>, BackendError> {
        self.saved_sessions_service().list_saved_sessions()
    }

    pub fn saved_session(&self, session_id: SessionId) -> Result<SavedNativeSession, BackendError> {
        self.saved_sessions_service().saved_session(session_id)
    }

    pub fn saved_session_v2_restore_plan(
        &self,
        session_id: SessionId,
    ) -> Result<Option<RestorePlan>, BackendError> {
        self.saved_sessions_service().saved_session_v2_restore_plan(session_id)
    }

    pub fn delete_saved_session(&self, session_id: SessionId) -> Result<(), BackendError> {
        self.saved_sessions_service().delete_saved_session(session_id)
    }

    pub fn prune_saved_sessions(
        &self,
        keep_latest: usize,
    ) -> Result<PrunedSavedSessions, BackendError> {
        self.saved_sessions_service().prune_saved_sessions(keep_latest)
    }

    pub async fn restore_saved_session(
        &self,
        session_id: SessionId,
    ) -> Result<BackendSessionSummary, BackendError> {
        self.saved_sessions_service().restore_saved_session(session_id).await
    }
}
