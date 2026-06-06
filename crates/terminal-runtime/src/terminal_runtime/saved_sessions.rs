use terminal_backend_api::{BackendError, BackendSessionSummary};
use terminal_domain::SessionId;
use terminal_persistence::{
    PrunedSavedSessions, RestorePlan, SavedNativeSession, SavedSessionSummary,
};

use super::TerminalRuntime;

impl TerminalRuntime {
    pub fn list_saved_sessions(&self) -> Result<Vec<SavedSessionSummary>, BackendError> {
        self.sessions.list_saved_sessions()
    }

    pub fn saved_session(&self, session_id: SessionId) -> Result<SavedNativeSession, BackendError> {
        self.sessions.saved_session(session_id)
    }

    pub fn saved_session_v2_restore_plan(
        &self,
        session_id: SessionId,
    ) -> Result<Option<RestorePlan>, BackendError> {
        self.sessions.saved_session_v2_restore_plan(session_id)
    }

    pub fn delete_saved_session(&self, session_id: SessionId) -> Result<(), BackendError> {
        self.sessions.delete_saved_session(session_id)
    }

    pub fn prune_saved_sessions(
        &self,
        keep_latest: usize,
    ) -> Result<PrunedSavedSessions, BackendError> {
        self.sessions.prune_saved_sessions(keep_latest)
    }

    pub async fn restore_saved_session(
        &self,
        session_id: SessionId,
    ) -> Result<BackendSessionSummary, BackendError> {
        self.sessions.restore_saved_session(session_id).await
    }
}
