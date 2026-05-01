use terminal_backend_api::BackendError;
use terminal_domain::SessionId;
use terminal_projection::SessionHealthSnapshot;

use super::SessionRuntime;

impl SessionRuntime<'_> {
    pub(in crate::sessions) fn session_health_snapshot(
        &self,
        session_id: SessionId,
    ) -> Result<SessionHealthSnapshot, BackendError> {
        self.registry
            .get(session_id)
            .map(|session| session.health)
            .ok_or_else(|| BackendError::not_found(format!("unknown session {session_id:?}")))
    }

    pub(in crate::sessions) fn record_session_health(
        &self,
        session_id: SessionId,
        health: SessionHealthSnapshot,
    ) {
        self.registry.update_health(session_id, health);
    }
}
