use terminal_backend_api::{BackendError, BackendSessionSummary};
use terminal_domain::SessionId;

use crate::application::{
    RuntimePrunedSavedSessions, RuntimeSavedSessionRecord, RuntimeSavedSessionSummary,
    TerminalDaemonSavedSessionsPort,
};

use super::{
    TerminalRuntimeAdapter,
    mappings::{map_saved_session_record, map_saved_session_summary},
};

impl TerminalDaemonSavedSessionsPort for TerminalRuntimeAdapter<'_> {
    fn list_saved_sessions(&self) -> Result<Vec<RuntimeSavedSessionSummary>, BackendError> {
        self.runtime.list_saved_sessions().map(|sessions| {
            sessions
                .into_iter()
                .map(|session| {
                    let restore_plan = self
                        .runtime
                        .saved_session_v2_restore_plan(session.session_id)
                        .ok()
                        .flatten();
                    map_saved_session_summary(session, restore_plan)
                })
                .collect()
        })
    }

    fn saved_session(
        &self,
        session_id: SessionId,
    ) -> Result<RuntimeSavedSessionRecord, BackendError> {
        let restore_plan = self.runtime.saved_session_v2_restore_plan(session_id).ok().flatten();
        self.runtime
            .saved_session(session_id)
            .map(|session| map_saved_session_record(session, restore_plan))
    }

    fn delete_saved_session(&self, session_id: SessionId) -> Result<(), BackendError> {
        self.runtime.delete_saved_session(session_id)
    }

    fn prune_saved_sessions(
        &self,
        keep_latest: usize,
    ) -> Result<RuntimePrunedSavedSessions, BackendError> {
        self.runtime.prune_saved_sessions(keep_latest).map(|pruned| RuntimePrunedSavedSessions {
            deleted_count: pruned.deleted_count,
            kept_count: pruned.kept_count,
        })
    }

    async fn restore_saved_session(
        &self,
        session_id: SessionId,
    ) -> Result<BackendSessionSummary, BackendError> {
        self.runtime.restore_saved_session(session_id).await
    }
}
