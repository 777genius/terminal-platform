use terminal_backend_api::{BackendError, MuxCommand, MuxCommandResult};
use terminal_domain::{PaneId, SessionId};
use terminal_persistence::{CommandHistoryEntryRecord, PaneHistoryHydrationRecord};
use terminal_projection::{ScreenDelta, ScreenSnapshot, SessionHealthSnapshot, TopologySnapshot};

use crate::application::TerminalDaemonActiveSessionPort;

use super::TerminalRuntimeAdapter;

impl TerminalDaemonActiveSessionPort for TerminalRuntimeAdapter<'_> {
    fn session_health_snapshot(
        &self,
        session_id: SessionId,
    ) -> Result<SessionHealthSnapshot, BackendError> {
        self.runtime.session_health_snapshot(session_id)
    }

    async fn topology_snapshot(
        &self,
        session_id: SessionId,
    ) -> Result<TopologySnapshot, BackendError> {
        self.runtime.topology_snapshot(session_id).await
    }

    async fn screen_snapshot(
        &self,
        session_id: SessionId,
        pane_id: PaneId,
    ) -> Result<ScreenSnapshot, BackendError> {
        self.runtime.screen_snapshot(session_id, pane_id).await
    }

    async fn screen_delta(
        &self,
        session_id: SessionId,
        pane_id: PaneId,
        from_sequence: u64,
    ) -> Result<ScreenDelta, BackendError> {
        self.runtime.screen_delta(session_id, pane_id, from_sequence).await
    }

    async fn pane_history(
        &self,
        session_id: SessionId,
        pane_id: PaneId,
        from_event_seq: Option<i64>,
        max_segments: Option<i64>,
        max_bytes: Option<i64>,
    ) -> Result<PaneHistoryHydrationRecord, BackendError> {
        self.runtime
            .pane_history(session_id, pane_id, from_event_seq, max_segments, max_bytes)
            .await
    }

    async fn command_history(
        &self,
        session_id: Option<SessionId>,
        limit: Option<i64>,
    ) -> Result<Vec<CommandHistoryEntryRecord>, BackendError> {
        self.runtime.command_history(session_id, limit).await
    }

    async fn dispatch(
        &self,
        session_id: SessionId,
        command: MuxCommand,
    ) -> Result<MuxCommandResult, BackendError> {
        self.runtime.dispatch(session_id, command).await
    }
}
