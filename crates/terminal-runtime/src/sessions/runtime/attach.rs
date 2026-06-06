use terminal_backend_api::{BackendError, BackendSessionPort};
use terminal_domain::SessionId;
use terminal_projection::SessionHealthSnapshot;

use super::{
    SessionRuntime,
    helpers::{saved_session_title, session_health_from_attach_error},
};

impl SessionRuntime<'_> {
    pub(in crate::sessions) async fn attach_session(
        &self,
        session_id: SessionId,
    ) -> Result<Box<dyn BackendSessionPort>, BackendError> {
        let descriptor = self
            .registry
            .get(session_id)
            .ok_or_else(|| BackendError::not_found(format!("unknown session {session_id:?}")))?;

        match self
            .backend(descriptor.route.backend)?
            .attach_session(descriptor.session_id, descriptor.route)
            .await
        {
            Ok(session) => {
                self.mark_session_ready(session_id);
                Ok(session)
            }
            Err(error) => {
                if let Some(health) = session_health_from_attach_error(session_id, &error) {
                    self.record_session_health(session_id, health);
                }
                Err(error)
            }
        }
    }

    pub(in crate::sessions) async fn refresh_session_summary_title(
        &self,
        session_id: SessionId,
        session: &dyn BackendSessionPort,
    ) {
        let Some(descriptor) = self.registry.get(session_id) else {
            return;
        };
        let Ok(topology) = session.topology_snapshot().await else {
            return;
        };
        self.registry.update_title(session_id, saved_session_title(descriptor.title, &topology));
    }

    pub(in crate::sessions) fn mark_session_ready(&self, session_id: SessionId) {
        self.registry.update_health(session_id, SessionHealthSnapshot::ready(session_id));
    }
}
