use terminal_protocol::{
    CreateSessionResponse, ListSessionsResponse, ProtocolError, RequestEnvelope, RequestPayload,
    ResponseEnvelope, ResponsePayload,
};

use crate::TerminalDaemonState;

#[derive(Default)]
pub struct TerminalDaemon {
    state: TerminalDaemonState,
}

impl TerminalDaemon {
    #[must_use]
    pub fn new(state: TerminalDaemonState) -> Self {
        Self { state }
    }

    pub async fn handle_request(
        &self,
        request: RequestEnvelope,
    ) -> Result<ResponseEnvelope, ProtocolError> {
        let payload = match request.payload {
            RequestPayload::Handshake => ResponsePayload::Handshake(self.state.handshake()),
            RequestPayload::CreateSession(request) => {
                let session = self
                    .state
                    .create_session(request.backend, request.spec)
                    .await
                    .map_err(|error| ProtocolError::new("backend_error", error.to_string()))?;

                ResponsePayload::CreateSession(CreateSessionResponse { session })
            }
            RequestPayload::ListSessions => ResponsePayload::ListSessions(ListSessionsResponse {
                sessions: self.state.list_sessions(),
            }),
            RequestPayload::GetTopologySnapshot(request) => ResponsePayload::TopologySnapshot(
                self.state
                    .topology_snapshot(request.session_id)
                    .await
                    .map_err(|error| ProtocolError::new("backend_error", error.to_string()))?,
            ),
            RequestPayload::GetScreenSnapshot(request) => ResponsePayload::ScreenSnapshot(
                self.state
                    .screen_snapshot(request.session_id, request.pane_id)
                    .await
                    .map_err(|error| ProtocolError::new("backend_error", error.to_string()))?,
            ),
            RequestPayload::OpenSubscription(_) => ResponsePayload::SubscriptionOpened,
        };

        Ok(ResponseEnvelope { operation_id: request.operation_id, payload })
    }
}

#[cfg(test)]
mod tests {
    use terminal_backend_api::CreateSessionSpec;
    use terminal_domain::OperationId;
    use terminal_protocol::{RequestEnvelope, RequestPayload, ResponsePayload};

    use super::TerminalDaemon;

    #[tokio::test(flavor = "multi_thread")]
    async fn routes_handshake_requests() {
        let daemon = TerminalDaemon::default();
        let response = daemon
            .handle_request(RequestEnvelope {
                operation_id: OperationId::new(),
                payload: RequestPayload::Handshake,
            })
            .await
            .expect("handshake routing should succeed");

        match response.payload {
            ResponsePayload::Handshake(handshake) => {
                assert_eq!(handshake.protocol_version.major, 0);
                assert_eq!(handshake.available_backends.len(), 3);
            }
            other => panic!("unexpected response payload: {other:?}"),
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn routes_native_session_creation_requests() {
        let daemon = TerminalDaemon::default();
        let response = daemon
            .handle_request(RequestEnvelope {
                operation_id: OperationId::new(),
                payload: RequestPayload::CreateSession(terminal_protocol::CreateSessionRequest {
                    backend: terminal_domain::BackendKind::Native,
                    spec: CreateSessionSpec { title: Some("shell".to_string()) },
                }),
            })
            .await
            .expect("create session routing should succeed");

        match response.payload {
            ResponsePayload::CreateSession(created) => {
                assert_eq!(created.session.route.backend, terminal_domain::BackendKind::Native);
                assert_eq!(created.session.title.as_deref(), Some("shell"));
            }
            other => panic!("unexpected response payload: {other:?}"),
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn routes_topology_snapshot_requests() {
        let daemon = TerminalDaemon::default();
        let created = daemon
            .handle_request(RequestEnvelope {
                operation_id: OperationId::new(),
                payload: RequestPayload::CreateSession(terminal_protocol::CreateSessionRequest {
                    backend: terminal_domain::BackendKind::Native,
                    spec: CreateSessionSpec { title: Some("shell".to_string()) },
                }),
            })
            .await
            .expect("create session routing should succeed");
        let session_id = match created.payload {
            ResponsePayload::CreateSession(created) => created.session.session_id,
            other => panic!("unexpected response payload: {other:?}"),
        };
        let response = daemon
            .handle_request(RequestEnvelope {
                operation_id: OperationId::new(),
                payload: RequestPayload::GetTopologySnapshot(
                    terminal_protocol::GetTopologySnapshotRequest { session_id },
                ),
            })
            .await
            .expect("topology routing should succeed");

        match response.payload {
            ResponsePayload::TopologySnapshot(topology) => {
                assert_eq!(topology.session_id, session_id);
                assert_eq!(topology.tabs.len(), 1);
            }
            other => panic!("unexpected response payload: {other:?}"),
        }
    }
}
