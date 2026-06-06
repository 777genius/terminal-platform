use terminal_protocol::{
    DispatchMuxCommandRequest, GetPaneHistoryRequest, GetScreenDeltaRequest,
    GetScreenSnapshotRequest, GetSessionHealthSnapshotRequest, GetTopologySnapshotRequest,
    ListCommandHistoryRequest, ProtocolError, ResponsePayload,
};

use crate::{
    adapters::{map_backend_error, map_command_history, map_pane_history},
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
    pub(super) fn session_health_snapshot_response(
        &self,
        request: GetSessionHealthSnapshotRequest,
    ) -> Result<ResponsePayload, ProtocolError> {
        Ok(ResponsePayload::SessionHealthSnapshot(
            self.active_sessions
                .session_health_snapshot(request.session_id)
                .map_err(map_backend_error)?,
        ))
    }

    pub(super) async fn topology_snapshot_response(
        &self,
        request: GetTopologySnapshotRequest,
    ) -> Result<ResponsePayload, ProtocolError> {
        Ok(ResponsePayload::TopologySnapshot(
            self.active_sessions
                .topology_snapshot(request.session_id)
                .await
                .map_err(map_backend_error)?,
        ))
    }

    pub(super) async fn screen_snapshot_response(
        &self,
        request: GetScreenSnapshotRequest,
    ) -> Result<ResponsePayload, ProtocolError> {
        Ok(ResponsePayload::ScreenSnapshot(
            self.active_sessions
                .screen_snapshot(request.session_id, request.pane_id)
                .await
                .map_err(map_backend_error)?,
        ))
    }

    pub(super) async fn screen_delta_response(
        &self,
        request: GetScreenDeltaRequest,
    ) -> Result<ResponsePayload, ProtocolError> {
        Ok(ResponsePayload::ScreenDelta(
            self.active_sessions
                .screen_delta(request.session_id, request.pane_id, request.from_sequence)
                .await
                .map_err(map_backend_error)?,
        ))
    }

    pub(super) async fn pane_history_response(
        &self,
        request: GetPaneHistoryRequest,
    ) -> Result<ResponsePayload, ProtocolError> {
        Ok(ResponsePayload::PaneHistory(map_pane_history(
            self.active_sessions
                .pane_history(
                    request.session_id,
                    request.pane_id,
                    request.from_event_seq,
                    request.max_segments,
                    request.max_bytes,
                )
                .await
                .map_err(map_backend_error)?,
        )?))
    }

    pub(super) async fn command_history_response(
        &self,
        request: ListCommandHistoryRequest,
    ) -> Result<ResponsePayload, ProtocolError> {
        Ok(ResponsePayload::CommandHistory(map_command_history(
            self.active_sessions
                .command_history(request.session_id, request.limit)
                .await
                .map_err(map_backend_error)?,
        )?))
    }

    pub(super) async fn dispatch_mux_command_response(
        &self,
        request: DispatchMuxCommandRequest,
    ) -> Result<ResponsePayload, ProtocolError> {
        Ok(ResponsePayload::DispatchMuxCommand(
            self.active_sessions
                .dispatch(request.session_id, request.command)
                .await
                .map_err(map_backend_error)?,
        ))
    }
}
