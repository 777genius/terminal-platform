use terminal_backend_api::BackendError;
use terminal_domain::{SessionId, SessionRoute};
use terminal_persistence::SessionRouteRecord;

use super::{SessionRuntime, helpers::session_route_fingerprint};

impl SessionRuntime<'_> {
    pub(in crate::sessions) fn resolve_session_id_for_route(
        &self,
        route: &SessionRoute,
    ) -> Result<SessionId, BackendError> {
        let route_fingerprint = session_route_fingerprint(route);
        if let Some(record) = self
            .persistence
            .load_session_route_by_fingerprint(&route_fingerprint)
            .map_err(|error| {
                BackendError::internal(format!(
                    "failed to load session route by fingerprint - {error}"
                ))
            })?
        {
            return Ok(record.session_id);
        }

        let session_id = SessionId::new();
        self.upsert_session_route(session_id, route)?;
        Ok(session_id)
    }

    pub(in crate::sessions) fn upsert_session_route(
        &self,
        session_id: SessionId,
        route: &SessionRoute,
    ) -> Result<(), BackendError> {
        self.persistence
            .upsert_session_route(&SessionRouteRecord {
                session_id,
                route: route.clone(),
                route_fingerprint: session_route_fingerprint(route),
            })
            .map_err(|error| {
                BackendError::internal(format!("failed to persist session route - {error}"))
            })
    }
}
