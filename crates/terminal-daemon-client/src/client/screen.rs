use terminal_domain::{PaneId, SessionId};
use terminal_projection::{ScreenDelta, ScreenSnapshot, SessionHealthSnapshot, TopologySnapshot};
use terminal_protocol::{
    GetScreenDeltaRequest, GetScreenSnapshotRequest, GetSessionHealthSnapshotRequest,
    GetTopologySnapshotRequest, ProtocolError, RequestPayload, ResponsePayload,
};

use super::LocalSocketDaemonClient;

impl LocalSocketDaemonClient {
    pub async fn session_health_snapshot(
        &self,
        session_id: SessionId,
    ) -> Result<SessionHealthSnapshot, ProtocolError> {
        let response = self
            .send_request(RequestPayload::GetSessionHealthSnapshot(
                GetSessionHealthSnapshotRequest { session_id },
            ))
            .await?;

        match response.payload {
            ResponsePayload::SessionHealthSnapshot(health) => Ok(health),
            other => Err(ProtocolError::unexpected_payload("session_health_snapshot", &other)),
        }
    }
    pub async fn topology_snapshot(
        &self,
        session_id: SessionId,
    ) -> Result<TopologySnapshot, ProtocolError> {
        let response = self
            .send_request(RequestPayload::GetTopologySnapshot(GetTopologySnapshotRequest {
                session_id,
            }))
            .await?;

        match response.payload {
            ResponsePayload::TopologySnapshot(snapshot) => Ok(snapshot),
            other => Err(ProtocolError::unexpected_payload("topology_snapshot", &other)),
        }
    }

    pub async fn screen_snapshot(
        &self,
        session_id: SessionId,
        pane_id: PaneId,
    ) -> Result<ScreenSnapshot, ProtocolError> {
        let response = self
            .send_request(RequestPayload::GetScreenSnapshot(GetScreenSnapshotRequest {
                session_id,
                pane_id,
            }))
            .await?;

        match response.payload {
            ResponsePayload::ScreenSnapshot(snapshot) => Ok(snapshot),
            other => Err(ProtocolError::unexpected_payload("screen_snapshot", &other)),
        }
    }

    pub async fn screen_delta(
        &self,
        session_id: SessionId,
        pane_id: PaneId,
        from_sequence: u64,
    ) -> Result<ScreenDelta, ProtocolError> {
        let response = self
            .send_request(RequestPayload::GetScreenDelta(GetScreenDeltaRequest {
                session_id,
                pane_id,
                from_sequence,
            }))
            .await?;

        match response.payload {
            ResponsePayload::ScreenDelta(delta) => Ok(delta),
            other => Err(ProtocolError::unexpected_payload("screen_delta", &other)),
        }
    }
}
