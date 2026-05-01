use terminal_protocol::{
    DeleteSavedSessionRequest, DeleteSavedSessionResponse, GetSavedSessionRequest,
    ListSavedSessionsResponse, ProtocolError, PruneSavedSessionsRequest,
    PruneSavedSessionsResponse, ResponsePayload, RestoreSavedSessionRequest,
    RestoreSavedSessionResponse, SavedSessionResponse,
};

use crate::{
    adapters::{
        map_backend_error, map_restore_saved_session_response, map_saved_session_record,
        map_saved_session_summary,
    },
    application::{
        TerminalDaemonActiveSessionPort, TerminalDaemonCatalogPort,
        TerminalDaemonSavedSessionsPort, TerminalDaemonSubscriptionPort,
    },
};

use super::TerminalDaemonRequestDispatcher;

impl<Catalog, SavedSessions, ActiveSessions, Subscriptions>
    TerminalDaemonRequestDispatcher<Catalog, SavedSessions, ActiveSessions, Subscriptions>
where
    Catalog: TerminalDaemonCatalogPort,
    SavedSessions: TerminalDaemonSavedSessionsPort,
    ActiveSessions: TerminalDaemonActiveSessionPort,
    Subscriptions: TerminalDaemonSubscriptionPort,
{
    pub(super) fn list_saved_sessions_response(&self) -> Result<ResponsePayload, ProtocolError> {
        Ok(ResponsePayload::ListSavedSessions(ListSavedSessionsResponse {
            sessions: self
                .saved_sessions
                .list_saved_sessions()
                .map_err(map_backend_error)?
                .into_iter()
                .map(map_saved_session_summary)
                .collect(),
        }))
    }

    pub(super) fn saved_session_response(
        &self,
        request: GetSavedSessionRequest,
    ) -> Result<ResponsePayload, ProtocolError> {
        Ok(ResponsePayload::SavedSession(SavedSessionResponse {
            session: map_saved_session_record(
                self.saved_sessions.saved_session(request.session_id).map_err(map_backend_error)?,
            ),
        }))
    }

    pub(super) fn delete_saved_session_response(
        &self,
        request: DeleteSavedSessionRequest,
    ) -> Result<ResponsePayload, ProtocolError> {
        self.saved_sessions.delete_saved_session(request.session_id).map_err(map_backend_error)?;

        Ok(ResponsePayload::DeleteSavedSession(DeleteSavedSessionResponse {
            session_id: request.session_id,
        }))
    }

    pub(super) fn prune_saved_sessions_response(
        &self,
        request: PruneSavedSessionsRequest,
    ) -> Result<ResponsePayload, ProtocolError> {
        let pruned = self
            .saved_sessions
            .prune_saved_sessions(request.keep_latest)
            .map_err(map_backend_error)?;

        Ok(ResponsePayload::PruneSavedSessions(PruneSavedSessionsResponse {
            deleted_count: pruned.deleted_count,
            kept_count: pruned.kept_count,
        }))
    }

    pub(super) async fn restore_saved_session_response(
        &self,
        request: RestoreSavedSessionRequest,
    ) -> Result<ResponsePayload, ProtocolError> {
        let saved =
            self.saved_sessions.saved_session(request.session_id).map_err(map_backend_error)?;
        let restored = self
            .saved_sessions
            .restore_saved_session(request.session_id)
            .await
            .map_err(map_backend_error)?;

        Ok(ResponsePayload::RestoreSavedSession(RestoreSavedSessionResponse {
            ..map_restore_saved_session_response(request.session_id, &saved, restored)
        }))
    }
}
