use terminal_backend_api::BackendError;
use terminal_domain::SessionId;
use terminal_projection::SessionHealthSnapshot;

use super::ActiveSessionService;

impl ActiveSessionService<'_> {
    pub(in crate::sessions) fn session_health_snapshot(
        &self,
        session_id: SessionId,
    ) -> Result<SessionHealthSnapshot, BackendError> {
        self.runtime.session_health_snapshot(session_id)
    }
}
