use terminal_backend_api::{BackendError, BackendErrorKind};
use terminal_protocol::{
    BackendCapabilitiesResponse, CreateSessionResponse, DiscoverSessionsResponse,
    ImportSessionResponse, ListSessionsResponse, OpenSubscriptionRequest, OpenSubscriptionResponse,
    ProtocolError, RequestEnvelope, RequestPayload, ResponseEnvelope, ResponsePayload,
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
                    .map_err(map_backend_error)?;

                ResponsePayload::CreateSession(CreateSessionResponse { session })
            }
            RequestPayload::ListSessions => ResponsePayload::ListSessions(ListSessionsResponse {
                sessions: self.state.list_sessions(),
            }),
            RequestPayload::DiscoverSessions(request) => {
                ResponsePayload::DiscoverSessions(DiscoverSessionsResponse {
                    sessions: self
                        .state
                        .discover_sessions(request.backend)
                        .await
                        .map_err(map_backend_error)?,
                })
            }
            RequestPayload::GetBackendCapabilities(request) => {
                ResponsePayload::BackendCapabilities(BackendCapabilitiesResponse {
                    backend: request.backend,
                    capabilities: self
                        .state
                        .backend_capabilities(request.backend)
                        .await
                        .map_err(map_backend_error)?,
                })
            }
            RequestPayload::ImportSession(request) => {
                let session = self
                    .state
                    .import_session(request.route, request.title)
                    .await
                    .map_err(map_backend_error)?;

                ResponsePayload::ImportSession(ImportSessionResponse { session })
            }
            RequestPayload::GetTopologySnapshot(request) => ResponsePayload::TopologySnapshot(
                self.state
                    .topology_snapshot(request.session_id)
                    .await
                    .map_err(map_backend_error)?,
            ),
            RequestPayload::GetScreenSnapshot(request) => ResponsePayload::ScreenSnapshot(
                self.state
                    .screen_snapshot(request.session_id, request.pane_id)
                    .await
                    .map_err(map_backend_error)?,
            ),
            RequestPayload::GetScreenDelta(request) => ResponsePayload::ScreenDelta(
                self.state
                    .screen_delta(request.session_id, request.pane_id, request.from_sequence)
                    .await
                    .map_err(map_backend_error)?,
            ),
            RequestPayload::DispatchMuxCommand(request) => ResponsePayload::DispatchMuxCommand(
                self.state
                    .dispatch(request.session_id, request.command)
                    .await
                    .map_err(map_backend_error)?,
            ),
            RequestPayload::OpenSubscription(request) => {
                ResponsePayload::SubscriptionOpened(OpenSubscriptionResponse {
                    subscription_id: self
                        .state
                        .open_subscription(request.session_id, request.spec)
                        .await
                        .map_err(map_backend_error)?
                        .subscription_id,
                })
            }
        };

        Ok(ResponseEnvelope { operation_id: request.operation_id, payload })
    }

    pub async fn open_subscription(
        &self,
        request: OpenSubscriptionRequest,
    ) -> Result<terminal_backend_api::BackendSubscription, ProtocolError> {
        self.state
            .open_subscription(request.session_id, request.spec)
            .await
            .map_err(map_backend_error)
    }
}

fn map_backend_error(error: BackendError) -> ProtocolError {
    let code = match error.kind {
        BackendErrorKind::Unsupported => "backend_unsupported",
        BackendErrorKind::NotFound => "backend_not_found",
        BackendErrorKind::InvalidInput => "backend_invalid_input",
        BackendErrorKind::Transport => "backend_transport",
        BackendErrorKind::Internal => "backend_internal",
    };

    ProtocolError::new(code, error.to_string())
}

#[cfg(test)]
mod tests {
    use terminal_backend_api::{CreateSessionSpec, MuxCommand, NewTabSpec, SubscriptionSpec};
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
                    spec: CreateSessionSpec {
                        title: Some("shell".to_string()),
                        ..CreateSessionSpec::default()
                    },
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
    async fn routes_backend_capabilities_requests() {
        let daemon = TerminalDaemon::default();
        let response = daemon
            .handle_request(RequestEnvelope {
                operation_id: OperationId::new(),
                payload: RequestPayload::GetBackendCapabilities(
                    terminal_protocol::GetBackendCapabilitiesRequest {
                        backend: terminal_domain::BackendKind::Native,
                    },
                ),
            })
            .await
            .expect("capabilities routing should succeed");

        match response.payload {
            ResponsePayload::BackendCapabilities(capabilities) => {
                assert_eq!(capabilities.backend, terminal_domain::BackendKind::Native);
                assert!(capabilities.capabilities.tiled_panes);
                assert!(capabilities.capabilities.tab_create);
                assert!(capabilities.capabilities.tab_close);
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
                    spec: CreateSessionSpec {
                        title: Some("shell".to_string()),
                        ..CreateSessionSpec::default()
                    },
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

    #[tokio::test(flavor = "multi_thread")]
    async fn routes_dispatch_mux_command_requests() {
        let daemon = TerminalDaemon::default();
        let created = daemon
            .handle_request(RequestEnvelope {
                operation_id: OperationId::new(),
                payload: RequestPayload::CreateSession(terminal_protocol::CreateSessionRequest {
                    backend: terminal_domain::BackendKind::Native,
                    spec: CreateSessionSpec {
                        title: Some("shell".to_string()),
                        ..CreateSessionSpec::default()
                    },
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
                payload: RequestPayload::DispatchMuxCommand(
                    terminal_protocol::DispatchMuxCommandRequest {
                        session_id,
                        command: MuxCommand::NewTab(NewTabSpec { title: Some("logs".to_string()) }),
                    },
                ),
            })
            .await
            .expect("dispatch routing should succeed");

        match response.payload {
            ResponsePayload::DispatchMuxCommand(result) => assert!(result.changed),
            other => panic!("unexpected response payload: {other:?}"),
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn routes_screen_delta_requests() {
        let daemon = TerminalDaemon::default();
        let created = daemon
            .handle_request(RequestEnvelope {
                operation_id: OperationId::new(),
                payload: RequestPayload::CreateSession(terminal_protocol::CreateSessionRequest {
                    backend: terminal_domain::BackendKind::Native,
                    spec: CreateSessionSpec {
                        title: Some("shell".to_string()),
                        ..CreateSessionSpec::default()
                    },
                }),
            })
            .await
            .expect("create session routing should succeed");
        let session_id = match created.payload {
            ResponsePayload::CreateSession(created) => created.session.session_id,
            other => panic!("unexpected response payload: {other:?}"),
        };
        let topology = daemon
            .handle_request(RequestEnvelope {
                operation_id: OperationId::new(),
                payload: RequestPayload::GetTopologySnapshot(
                    terminal_protocol::GetTopologySnapshotRequest { session_id },
                ),
            })
            .await
            .expect("topology routing should succeed");
        let pane_id = match topology.payload {
            ResponsePayload::TopologySnapshot(topology) => {
                topology.tabs[0].focused_pane.expect("focused pane should exist")
            }
            other => panic!("unexpected response payload: {other:?}"),
        };
        let response = daemon
            .handle_request(RequestEnvelope {
                operation_id: OperationId::new(),
                payload: RequestPayload::GetScreenDelta(terminal_protocol::GetScreenDeltaRequest {
                    session_id,
                    pane_id,
                    from_sequence: 0,
                }),
            })
            .await
            .expect("screen delta routing should succeed");

        match response.payload {
            ResponsePayload::ScreenDelta(delta) => {
                assert_eq!(delta.pane_id, pane_id);
                assert_eq!(delta.from_sequence, 0);
                assert!(delta.to_sequence >= delta.from_sequence);
            }
            other => panic!("unexpected response payload: {other:?}"),
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn routes_open_subscription_requests() {
        let daemon = TerminalDaemon::default();
        let created = daemon
            .handle_request(RequestEnvelope {
                operation_id: OperationId::new(),
                payload: RequestPayload::CreateSession(terminal_protocol::CreateSessionRequest {
                    backend: terminal_domain::BackendKind::Native,
                    spec: CreateSessionSpec {
                        title: Some("shell".to_string()),
                        ..CreateSessionSpec::default()
                    },
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
                payload: RequestPayload::OpenSubscription(
                    terminal_protocol::OpenSubscriptionRequest {
                        session_id,
                        spec: SubscriptionSpec::SessionTopology,
                    },
                ),
            })
            .await
            .expect("subscription routing should succeed");

        match response.payload {
            ResponsePayload::SubscriptionOpened(opened) => {
                let _subscription_id = opened.subscription_id;
            }
            other => panic!("unexpected response payload: {other:?}"),
        }
    }
}
