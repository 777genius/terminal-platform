use terminal_backend_api::{BackendError, BackendErrorKind};
use terminal_protocol::{
    BackendCapabilitiesResponse, CreateSessionResponse, DeleteSavedSessionResponse,
    DiscoverSessionsResponse, ImportSessionResponse, ListSavedSessionsResponse,
    ListSessionsResponse, OpenSubscriptionRequest, OpenSubscriptionResponse, ProtocolError,
    PruneSavedSessionsResponse, RequestEnvelope, RequestPayload, ResponseEnvelope, ResponsePayload,
    RestoreSavedSessionResponse, SavedSessionRecord, SavedSessionResponse,
    SavedSessionRestoreSemantics, SavedSessionSummary,
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
            RequestPayload::ListSavedSessions => {
                ResponsePayload::ListSavedSessions(ListSavedSessionsResponse {
                    sessions: self
                        .state
                        .list_saved_sessions()
                        .map_err(map_backend_error)?
                        .into_iter()
                        .map(map_saved_session_summary)
                        .collect(),
                })
            }
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
            RequestPayload::GetSavedSession(request) => {
                ResponsePayload::SavedSession(SavedSessionResponse {
                    session: map_saved_session_record(
                        self.state.saved_session(request.session_id).map_err(map_backend_error)?,
                    ),
                })
            }
            RequestPayload::DeleteSavedSession(request) => {
                self.state.delete_saved_session(request.session_id).map_err(map_backend_error)?;
                ResponsePayload::DeleteSavedSession(DeleteSavedSessionResponse {
                    session_id: request.session_id,
                })
            }
            RequestPayload::PruneSavedSessions(request) => {
                let pruned = self
                    .state
                    .prune_saved_sessions(request.keep_latest)
                    .map_err(map_backend_error)?;
                ResponsePayload::PruneSavedSessions(PruneSavedSessionsResponse {
                    deleted_count: pruned.deleted_count,
                    kept_count: pruned.kept_count,
                })
            }
            RequestPayload::RestoreSavedSession(request) => {
                let saved =
                    self.state.saved_session(request.session_id).map_err(map_backend_error)?;
                ResponsePayload::RestoreSavedSession(RestoreSavedSessionResponse {
                    session: self
                        .state
                        .restore_saved_session(request.session_id)
                        .await
                        .map_err(map_backend_error)?,
                    saved_session_id: request.session_id,
                    manifest: saved.manifest.clone(),
                    restore_semantics: saved_session_restore_semantics(saved.launch.is_some()),
                })
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
    let message = error.to_string();

    match error.degraded_reason {
        Some(degraded_reason) => {
            ProtocolError::with_degraded_reason(code, message, degraded_reason)
        }
        None => ProtocolError::new(code, message),
    }
}

fn map_saved_session_summary(
    session: terminal_persistence::SavedSessionSummary,
) -> SavedSessionSummary {
    SavedSessionSummary {
        session_id: session.session_id,
        route: session.route,
        title: session.title,
        saved_at_ms: session.saved_at_ms,
        manifest: session.manifest,
        has_launch: session.has_launch,
        tab_count: session.tab_count,
        pane_count: session.pane_count,
        restore_semantics: saved_session_restore_semantics(session.has_launch),
    }
}

fn map_saved_session_record(
    session: terminal_persistence::SavedNativeSession,
) -> SavedSessionRecord {
    let has_launch = session.launch.is_some();
    SavedSessionRecord {
        session_id: session.session_id,
        route: session.route,
        title: session.title,
        launch: session.launch,
        manifest: session.manifest,
        topology: session.topology,
        screens: session.screens,
        saved_at_ms: session.saved_at_ms,
        restore_semantics: saved_session_restore_semantics(has_launch),
    }
}

fn saved_session_restore_semantics(has_launch: bool) -> SavedSessionRestoreSemantics {
    SavedSessionRestoreSemantics {
        restores_topology: true,
        restores_focus_state: true,
        restores_tab_titles: true,
        uses_saved_launch_spec: has_launch,
        replays_saved_screen_buffers: false,
        preserves_process_state: false,
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use terminal_backend_api::{CreateSessionSpec, MuxCommand, NewTabSpec, SubscriptionSpec};
    use terminal_domain::{
        CURRENT_BINARY_VERSION, CURRENT_PROTOCOL_MAJOR, CURRENT_PROTOCOL_MINOR, OperationId,
    };
    use terminal_persistence::SqliteSessionStore;
    use terminal_protocol::{RequestEnvelope, RequestPayload, ResponsePayload};

    use super::TerminalDaemon;

    fn isolated_daemon() -> TerminalDaemon {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let store = SqliteSessionStore::open(std::env::temp_dir().join(format!(
            "terminal-platform-daemon-service-{}-{nanos}.sqlite3",
            std::process::id()
        )))
        .expect("isolated sqlite session store should open");

        TerminalDaemon::new(crate::TerminalDaemonState::with_default_persistence(store))
    }

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
                assert!(capabilities.capabilities.tab_focus);
            }
            other => panic!("unexpected response payload: {other:?}"),
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn routes_saved_session_requests() {
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
        let saved = daemon
            .handle_request(RequestEnvelope {
                operation_id: OperationId::new(),
                payload: RequestPayload::DispatchMuxCommand(
                    terminal_protocol::DispatchMuxCommandRequest {
                        session_id,
                        command: terminal_backend_api::MuxCommand::SaveSession,
                    },
                ),
            })
            .await
            .expect("save routing should succeed");
        match saved.payload {
            ResponsePayload::DispatchMuxCommand(result) => assert!(!result.changed),
            other => panic!("unexpected response payload: {other:?}"),
        }

        let listed = daemon
            .handle_request(RequestEnvelope {
                operation_id: OperationId::new(),
                payload: RequestPayload::ListSavedSessions,
            })
            .await
            .expect("list saved sessions routing should succeed");
        let loaded = daemon
            .handle_request(RequestEnvelope {
                operation_id: OperationId::new(),
                payload: RequestPayload::GetSavedSession(
                    terminal_protocol::GetSavedSessionRequest { session_id },
                ),
            })
            .await
            .expect("get saved session routing should succeed");

        match listed.payload {
            ResponsePayload::ListSavedSessions(listed) => {
                let session = listed
                    .sessions
                    .iter()
                    .find(|session| session.session_id == session_id)
                    .expect("saved session should be listed");
                assert_eq!(session.manifest.format_version, 1);
                assert_eq!(session.manifest.binary_version, CURRENT_BINARY_VERSION);
                assert_eq!(session.manifest.protocol_major, CURRENT_PROTOCOL_MAJOR);
                assert_eq!(session.manifest.protocol_minor, CURRENT_PROTOCOL_MINOR);
                assert!(session.restore_semantics.restores_topology);
                assert!(!session.restore_semantics.uses_saved_launch_spec);
                assert!(!session.restore_semantics.replays_saved_screen_buffers);
                assert!(!session.restore_semantics.preserves_process_state);
            }
            other => panic!("unexpected response payload: {other:?}"),
        }
        match loaded.payload {
            ResponsePayload::SavedSession(saved) => {
                assert_eq!(saved.session.session_id, session_id);
                assert_eq!(saved.session.route.backend, terminal_domain::BackendKind::Native);
                assert_eq!(saved.session.manifest.binary_version, CURRENT_BINARY_VERSION);
                assert!(saved.session.restore_semantics.restores_focus_state);
                assert!(saved.session.restore_semantics.restores_tab_titles);
                assert!(!saved.session.restore_semantics.replays_saved_screen_buffers);
                assert!(!saved.session.restore_semantics.preserves_process_state);
            }
            other => panic!("unexpected response payload: {other:?}"),
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn routes_delete_saved_session_requests() {
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
        daemon
            .handle_request(RequestEnvelope {
                operation_id: OperationId::new(),
                payload: RequestPayload::DispatchMuxCommand(
                    terminal_protocol::DispatchMuxCommandRequest {
                        session_id,
                        command: terminal_backend_api::MuxCommand::SaveSession,
                    },
                ),
            })
            .await
            .expect("save routing should succeed");

        let deleted = daemon
            .handle_request(RequestEnvelope {
                operation_id: OperationId::new(),
                payload: RequestPayload::DeleteSavedSession(
                    terminal_protocol::DeleteSavedSessionRequest { session_id },
                ),
            })
            .await
            .expect("delete saved session routing should succeed");

        match deleted.payload {
            ResponsePayload::DeleteSavedSession(deleted) => {
                assert_eq!(deleted.session_id, session_id);
            }
            other => panic!("unexpected response payload: {other:?}"),
        }

        let lookup_error = daemon
            .handle_request(RequestEnvelope {
                operation_id: OperationId::new(),
                payload: RequestPayload::GetSavedSession(
                    terminal_protocol::GetSavedSessionRequest { session_id },
                ),
            })
            .await
            .expect_err("deleted saved session lookup should fail");
        assert_eq!(lookup_error.code, "backend_not_found");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn routes_restore_saved_session_requests() {
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
        daemon
            .handle_request(RequestEnvelope {
                operation_id: OperationId::new(),
                payload: RequestPayload::DispatchMuxCommand(
                    terminal_protocol::DispatchMuxCommandRequest {
                        session_id,
                        command: terminal_backend_api::MuxCommand::SaveSession,
                    },
                ),
            })
            .await
            .expect("save routing should succeed");

        let restored = daemon
            .handle_request(RequestEnvelope {
                operation_id: OperationId::new(),
                payload: RequestPayload::RestoreSavedSession(
                    terminal_protocol::RestoreSavedSessionRequest { session_id },
                ),
            })
            .await
            .expect("restore routing should succeed");

        match restored.payload {
            ResponsePayload::RestoreSavedSession(restored) => {
                assert_eq!(restored.saved_session_id, session_id);
                assert_ne!(restored.session.session_id, session_id);
                assert_eq!(restored.session.route.backend, terminal_domain::BackendKind::Native);
                assert_eq!(restored.manifest.binary_version, CURRENT_BINARY_VERSION);
                assert!(restored.restore_semantics.restores_topology);
                assert!(!restored.restore_semantics.uses_saved_launch_spec);
                assert!(!restored.restore_semantics.replays_saved_screen_buffers);
                assert!(!restored.restore_semantics.preserves_process_state);
            }
            other => panic!("unexpected response payload: {other:?}"),
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn routes_prune_saved_sessions_requests() {
        let daemon = isolated_daemon();
        for title in ["shell-a", "shell-b", "shell-c"] {
            let created = daemon
                .handle_request(RequestEnvelope {
                    operation_id: OperationId::new(),
                    payload: RequestPayload::CreateSession(
                        terminal_protocol::CreateSessionRequest {
                            backend: terminal_domain::BackendKind::Native,
                            spec: CreateSessionSpec {
                                title: Some(title.to_string()),
                                ..CreateSessionSpec::default()
                            },
                        },
                    ),
                })
                .await
                .expect("create session routing should succeed");
            let session_id = match created.payload {
                ResponsePayload::CreateSession(created) => created.session.session_id,
                other => panic!("unexpected response payload: {other:?}"),
            };
            daemon
                .handle_request(RequestEnvelope {
                    operation_id: OperationId::new(),
                    payload: RequestPayload::DispatchMuxCommand(
                        terminal_protocol::DispatchMuxCommandRequest {
                            session_id,
                            command: terminal_backend_api::MuxCommand::SaveSession,
                        },
                    ),
                })
                .await
                .expect("save routing should succeed");
        }

        let pruned = daemon
            .handle_request(RequestEnvelope {
                operation_id: OperationId::new(),
                payload: RequestPayload::PruneSavedSessions(
                    terminal_protocol::PruneSavedSessionsRequest { keep_latest: 1 },
                ),
            })
            .await
            .expect("prune saved sessions routing should succeed");
        let listed = daemon
            .handle_request(RequestEnvelope {
                operation_id: OperationId::new(),
                payload: RequestPayload::ListSavedSessions,
            })
            .await
            .expect("list saved sessions routing should succeed");

        match pruned.payload {
            ResponsePayload::PruneSavedSessions(pruned) => {
                assert_eq!(pruned.deleted_count, 2);
                assert_eq!(pruned.kept_count, 1);
            }
            other => panic!("unexpected response payload: {other:?}"),
        }
        match listed.payload {
            ResponsePayload::ListSavedSessions(listed) => {
                assert_eq!(listed.sessions.len(), 1);
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
