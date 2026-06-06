use terminal_protocol::{OpenSubscriptionRequest, ProtocolError, RequestPayload, ResponsePayload};

use crate::application::{
    TerminalDaemonActiveSessionPort, TerminalDaemonCatalogPort, TerminalDaemonSavedSessionsPort,
    TerminalDaemonSubscriptionPort,
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
    pub(super) async fn dispatch_payload(
        &self,
        payload: RequestPayload,
    ) -> Result<ResponsePayload, ProtocolError> {
        match payload {
            RequestPayload::Handshake => Ok(self.handshake_response()),
            RequestPayload::CreateSession(request) => self.create_session_response(request).await,
            RequestPayload::ListSessions => Ok(self.list_sessions_response()),
            RequestPayload::ListSavedSessions => self.list_saved_sessions_response(),
            RequestPayload::DiscoverSessions(request) => {
                self.discover_sessions_response(request).await
            }
            RequestPayload::GetBackendCapabilities(request) => {
                self.backend_capabilities_response(request).await
            }
            RequestPayload::ImportSession(request) => self.import_session_response(request).await,
            RequestPayload::GetSavedSession(request) => self.saved_session_response(request),
            RequestPayload::DeleteSavedSession(request) => {
                self.delete_saved_session_response(request)
            }
            RequestPayload::PruneSavedSessions(request) => {
                self.prune_saved_sessions_response(request)
            }
            RequestPayload::RestoreSavedSession(request) => {
                self.restore_saved_session_response(request).await
            }
            RequestPayload::GetSessionHealthSnapshot(request) => {
                self.session_health_snapshot_response(request)
            }
            RequestPayload::GetTopologySnapshot(request) => {
                self.topology_snapshot_response(request).await
            }
            RequestPayload::GetScreenSnapshot(request) => {
                self.screen_snapshot_response(request).await
            }
            RequestPayload::GetScreenDelta(request) => self.screen_delta_response(request).await,
            RequestPayload::GetPaneHistory(request) => self.pane_history_response(request).await,
            RequestPayload::ListCommandHistory(request) => {
                self.command_history_response(request).await
            }
            RequestPayload::DispatchMuxCommand(request) => {
                self.dispatch_mux_command_response(request).await
            }
            RequestPayload::OpenSubscription(request) => {
                self.open_subscription_response(request).await
            }
        }
    }

    async fn open_subscription_response(
        &self,
        request: OpenSubscriptionRequest,
    ) -> Result<ResponsePayload, ProtocolError> {
        Ok(ResponsePayload::SubscriptionOpened(
            self.subscriptions.open_subscription_response(request).await?,
        ))
    }
}
