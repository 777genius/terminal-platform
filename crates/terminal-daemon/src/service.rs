use terminal_backend_api::BackendSubscription;
use terminal_persistence::SqliteSessionStore;
use terminal_protocol::{
    OpenSubscriptionRequest, ProtocolError, RequestEnvelope, ResponseEnvelope,
};
use terminal_runtime::TerminalRuntime;
use terminal_transport::TransportSubscription;

use crate::{
    adapters::TerminalRuntimeAdapter,
    application::{
        TerminalDaemonCatalogPort, TerminalDaemonRequestDispatcher,
        TerminalDaemonSubscriptionService,
    },
    composition, transport,
};

pub struct TerminalDaemon {
    runtime: TerminalRuntime,
}

impl Default for TerminalDaemon {
    fn default() -> Self {
        Self::new(composition::default_runtime())
    }
}

impl TerminalDaemon {
    #[must_use]
    pub fn new(runtime: TerminalRuntime) -> Self {
        Self { runtime }
    }

    #[must_use]
    pub fn with_persistence(persistence: SqliteSessionStore) -> Self {
        Self::new(composition::runtime_with_persistence(persistence))
    }

    pub async fn handle_request(
        &self,
        request: RequestEnvelope,
    ) -> Result<ResponseEnvelope, ProtocolError> {
        self.dispatcher().handle_request(request).await
    }

    pub async fn open_subscription(
        &self,
        request: OpenSubscriptionRequest,
    ) -> Result<BackendSubscription, ProtocolError> {
        self.subscription_service().open_backend_subscription(request).await
    }

    #[must_use]
    pub fn handshake(&self) -> terminal_protocol::Handshake {
        self.runtime_adapter().handshake()
    }

    #[must_use]
    pub fn session_count(&self) -> usize {
        self.runtime.session_count()
    }

    pub(crate) async fn open_transport_subscription(
        &self,
        request: OpenSubscriptionRequest,
    ) -> Result<TransportSubscription, ProtocolError> {
        Ok(transport::backend_subscription_to_transport(self.open_subscription(request).await?)
            .await)
    }

    fn runtime_adapter(&self) -> TerminalRuntimeAdapter<'_> {
        TerminalRuntimeAdapter::new(&self.runtime)
    }

    fn dispatcher(
        &self,
    ) -> TerminalDaemonRequestDispatcher<
        TerminalRuntimeAdapter<'_>,
        TerminalRuntimeAdapter<'_>,
        TerminalRuntimeAdapter<'_>,
        TerminalRuntimeAdapter<'_>,
    > {
        let runtime = self.runtime_adapter();
        TerminalDaemonRequestDispatcher::new(
            runtime,
            runtime,
            runtime,
            TerminalDaemonSubscriptionService::new(runtime),
        )
    }

    fn subscription_service(
        &self,
    ) -> TerminalDaemonSubscriptionService<TerminalRuntimeAdapter<'_>> {
        TerminalDaemonSubscriptionService::new(self.runtime_adapter())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use terminal_backend_api::{
        CreateSessionSpec, MuxCommand, NewTabSpec, ShellLaunchSpec, SubscriptionSpec,
    };
    use terminal_domain::{
        CURRENT_BINARY_VERSION, CURRENT_PROTOCOL_MAJOR, CURRENT_PROTOCOL_MINOR, OperationId,
        SavedSessionCompatibilityStatus, SavedSessionManifest, local_native_route,
    };
    use terminal_mux_domain::{PaneTreeNode, TabSnapshot};
    use terminal_persistence::SqliteSessionStore;
    use terminal_projection::TopologySnapshot;
    use terminal_protocol::{RequestEnvelope, RequestPayload, ResponsePayload};

    use super::TerminalDaemon;

    fn isolated_store_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "terminal-platform-daemon-service-{label}-{}-{}.sqlite3",
            std::process::id(),
            terminal_domain::SessionId::new().0
        ))
    }

    fn cat_launch_spec() -> ShellLaunchSpec {
        #[cfg(unix)]
        {
            ShellLaunchSpec::new("/bin/sh").with_args(["-lc", "printf 'ready\\n'; exec cat"])
        }

        #[cfg(windows)]
        {
            let program = std::env::var("COMSPEC")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "cmd.exe".to_string());

            ShellLaunchSpec::new(program).with_args(["/D", "/Q", "/K", "echo ready"])
        }
    }

    fn isolated_daemon() -> TerminalDaemon {
        let store = SqliteSessionStore::open(isolated_store_path("test"))
            .expect("isolated sqlite session store should open");

        TerminalDaemon::with_persistence(store)
    }

    fn save_incompatible_snapshot(
        label: &str,
        manifest: SavedSessionManifest,
    ) -> (TerminalDaemon, terminal_domain::SessionId) {
        let path = isolated_store_path(label);
        let store =
            SqliteSessionStore::open(&path).expect("isolated sqlite session store should open");
        let session_id = terminal_domain::SessionId::new();
        let tab_id = terminal_domain::TabId::new();
        let pane_id = terminal_domain::PaneId::new();
        store
            .save_native_session(&terminal_persistence::SavedNativeSession {
                session_id,
                route: local_native_route(session_id),
                title: Some("future-shell".to_string()),
                launch: None,
                manifest,
                topology: TopologySnapshot {
                    session_id,
                    backend_kind: terminal_domain::BackendKind::Native,
                    tabs: vec![TabSnapshot {
                        tab_id,
                        title: Some("future-shell".to_string()),
                        root: PaneTreeNode::Leaf { pane_id },
                        focused_pane: Some(pane_id),
                    }],
                    focused_tab: Some(tab_id),
                },
                screens: Vec::new(),
                saved_at_ms: SqliteSessionStore::save_timestamp_ms()
                    .expect("save timestamp should resolve"),
            })
            .expect("future snapshot should save");

        (TerminalDaemon::with_persistence(store), session_id)
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
                assert_eq!(handshake.daemon_phase, terminal_protocol::DaemonPhase::Ready);
                assert_eq!(
                    handshake.available_backends,
                    vec![
                        terminal_domain::BackendKind::Native,
                        terminal_domain::BackendKind::Tmux,
                        terminal_domain::BackendKind::Zellij
                    ]
                );
                assert_eq!(handshake.binary_version, CURRENT_BINARY_VERSION.to_string());
                assert_eq!(handshake.protocol_version.major, CURRENT_PROTOCOL_MAJOR);
                assert_eq!(handshake.protocol_version.minor, CURRENT_PROTOCOL_MINOR);
                assert!(handshake.capabilities.request_reply);
                assert!(handshake.capabilities.topology_subscriptions);
                assert!(handshake.capabilities.pane_subscriptions);
                assert!(handshake.capabilities.backend_discovery);
                assert!(handshake.capabilities.backend_capability_queries);
                assert!(handshake.capabilities.saved_sessions);
                assert!(handshake.capabilities.session_restore);
                assert!(handshake.capabilities.degraded_error_reasons);
                assert!(handshake.capabilities.session_health);
            }
            other => panic!("unexpected payload: {other:?}"),
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn routes_create_and_list_session_requests() {
        let daemon = isolated_daemon();
        let create = daemon
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
            .expect("create session should succeed");

        match create.payload {
            ResponsePayload::CreateSession(created) => {
                assert_eq!(created.session.title.as_deref(), Some("shell"));
                assert_eq!(created.session.route.backend, terminal_domain::BackendKind::Native);
            }
            other => panic!("unexpected payload: {other:?}"),
        }

        let listed = daemon
            .handle_request(RequestEnvelope {
                operation_id: OperationId::new(),
                payload: RequestPayload::ListSessions,
            })
            .await
            .expect("list sessions should succeed");

        match listed.payload {
            ResponsePayload::ListSessions(list) => {
                assert_eq!(list.sessions.len(), 1);
                assert_eq!(list.sessions[0].title.as_deref(), Some("shell"));
            }
            other => panic!("unexpected payload: {other:?}"),
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn routes_list_and_get_saved_session_requests() {
        let daemon = isolated_daemon();
        let created = daemon
            .handle_request(RequestEnvelope {
                operation_id: OperationId::new(),
                payload: RequestPayload::CreateSession(terminal_protocol::CreateSessionRequest {
                    backend: terminal_domain::BackendKind::Native,
                    spec: CreateSessionSpec {
                        title: Some("persisted-shell".to_string()),
                        launch: Some(cat_launch_spec()),
                    },
                }),
            })
            .await
            .expect("create session should succeed");
        let session_id = match created.payload {
            ResponsePayload::CreateSession(created) => created.session.session_id,
            other => panic!("unexpected payload: {other:?}"),
        };

        let saved = daemon
            .handle_request(RequestEnvelope {
                operation_id: OperationId::new(),
                payload: RequestPayload::DispatchMuxCommand(
                    terminal_protocol::DispatchMuxCommandRequest {
                        session_id,
                        command: MuxCommand::SaveSession,
                    },
                ),
            })
            .await
            .expect("save session should succeed");
        assert!(matches!(saved.payload, ResponsePayload::DispatchMuxCommand(_)));

        let list = daemon
            .handle_request(RequestEnvelope {
                operation_id: OperationId::new(),
                payload: RequestPayload::ListSavedSessions,
            })
            .await
            .expect("list saved sessions should succeed");
        let get = daemon
            .handle_request(RequestEnvelope {
                operation_id: OperationId::new(),
                payload: RequestPayload::GetSavedSession(
                    terminal_protocol::GetSavedSessionRequest { session_id },
                ),
            })
            .await
            .expect("get saved session should succeed");

        match list.payload {
            ResponsePayload::ListSavedSessions(listed) => {
                assert_eq!(listed.sessions.len(), 1);
                assert_eq!(listed.sessions[0].session_id, session_id);
                assert_eq!(listed.sessions[0].title.as_deref(), Some("persisted-shell"));
                assert_eq!(
                    listed.sessions[0].compatibility.status,
                    SavedSessionCompatibilityStatus::Compatible
                );
                assert!(listed.sessions[0].compatibility.can_restore);
                assert!(listed.sessions[0].restore_semantics.restores_topology);
                assert!(listed.sessions[0].restore_semantics.restores_focus_state);
                assert!(listed.sessions[0].restore_semantics.restores_tab_titles);
                assert!(listed.sessions[0].restore_semantics.uses_saved_launch_spec);
                assert!(!listed.sessions[0].restore_semantics.replays_saved_screen_buffers);
                assert!(!listed.sessions[0].restore_semantics.preserves_process_state);
            }
            other => panic!("unexpected payload: {other:?}"),
        }

        match get.payload {
            ResponsePayload::SavedSession(saved) => {
                assert_eq!(saved.session.session_id, session_id);
                assert_eq!(saved.session.title.as_deref(), Some("persisted-shell"));
                assert_eq!(
                    saved.session.compatibility.status,
                    SavedSessionCompatibilityStatus::Compatible
                );
                assert!(saved.session.compatibility.can_restore);
                assert!(saved.session.restore_semantics.restores_topology);
                assert!(saved.session.restore_semantics.restores_focus_state);
                assert!(saved.session.restore_semantics.restores_tab_titles);
                assert!(saved.session.restore_semantics.uses_saved_launch_spec);
                assert!(!saved.session.restore_semantics.replays_saved_screen_buffers);
                assert!(!saved.session.restore_semantics.preserves_process_state);
            }
            other => panic!("unexpected payload: {other:?}"),
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn routes_delete_saved_session_requests() {
        let daemon = isolated_daemon();
        let created = daemon
            .handle_request(RequestEnvelope {
                operation_id: OperationId::new(),
                payload: RequestPayload::CreateSession(terminal_protocol::CreateSessionRequest {
                    backend: terminal_domain::BackendKind::Native,
                    spec: CreateSessionSpec {
                        title: Some("delete-shell".to_string()),
                        ..CreateSessionSpec::default()
                    },
                }),
            })
            .await
            .expect("create session should succeed");
        let session_id = match created.payload {
            ResponsePayload::CreateSession(created) => created.session.session_id,
            other => panic!("unexpected payload: {other:?}"),
        };

        daemon
            .handle_request(RequestEnvelope {
                operation_id: OperationId::new(),
                payload: RequestPayload::DispatchMuxCommand(
                    terminal_protocol::DispatchMuxCommandRequest {
                        session_id,
                        command: MuxCommand::SaveSession,
                    },
                ),
            })
            .await
            .expect("save session should succeed");

        let deleted = daemon
            .handle_request(RequestEnvelope {
                operation_id: OperationId::new(),
                payload: RequestPayload::DeleteSavedSession(
                    terminal_protocol::DeleteSavedSessionRequest { session_id },
                ),
            })
            .await
            .expect("delete saved session should succeed");

        match deleted.payload {
            ResponsePayload::DeleteSavedSession(response) => {
                assert_eq!(response.session_id, session_id);
            }
            other => panic!("unexpected payload: {other:?}"),
        }

        let missing = daemon
            .handle_request(RequestEnvelope {
                operation_id: OperationId::new(),
                payload: RequestPayload::GetSavedSession(
                    terminal_protocol::GetSavedSessionRequest { session_id },
                ),
            })
            .await;
        assert!(missing.is_err());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn routes_restore_saved_session_requests() {
        let daemon = isolated_daemon();
        let created = daemon
            .handle_request(RequestEnvelope {
                operation_id: OperationId::new(),
                payload: RequestPayload::CreateSession(terminal_protocol::CreateSessionRequest {
                    backend: terminal_domain::BackendKind::Native,
                    spec: CreateSessionSpec {
                        title: Some("restore-shell".to_string()),
                        launch: Some(cat_launch_spec()),
                    },
                }),
            })
            .await
            .expect("create session should succeed");
        let session_id = match created.payload {
            ResponsePayload::CreateSession(created) => created.session.session_id,
            other => panic!("unexpected payload: {other:?}"),
        };

        daemon
            .handle_request(RequestEnvelope {
                operation_id: OperationId::new(),
                payload: RequestPayload::DispatchMuxCommand(
                    terminal_protocol::DispatchMuxCommandRequest {
                        session_id,
                        command: MuxCommand::SaveSession,
                    },
                ),
            })
            .await
            .expect("save session should succeed");

        let restored = daemon
            .handle_request(RequestEnvelope {
                operation_id: OperationId::new(),
                payload: RequestPayload::RestoreSavedSession(
                    terminal_protocol::RestoreSavedSessionRequest { session_id },
                ),
            })
            .await
            .expect("restore saved session should succeed");

        match restored.payload {
            ResponsePayload::RestoreSavedSession(response) => {
                assert_eq!(response.saved_session_id, session_id);
                assert_eq!(response.session.route.backend, terminal_domain::BackendKind::Native);
                assert_eq!(
                    response.compatibility.status,
                    SavedSessionCompatibilityStatus::Compatible
                );
                assert!(response.compatibility.can_restore);
                assert!(response.restore_semantics.restores_topology);
                assert!(response.restore_semantics.restores_focus_state);
                assert!(response.restore_semantics.restores_tab_titles);
                assert!(response.restore_semantics.uses_saved_launch_spec);
                assert!(!response.restore_semantics.replays_saved_screen_buffers);
                assert!(!response.restore_semantics.preserves_process_state);
            }
            other => panic!("unexpected payload: {other:?}"),
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn routes_prune_saved_sessions_requests() {
        let daemon = isolated_daemon();

        for label in ["one", "two", "three"] {
            let created = daemon
                .handle_request(RequestEnvelope {
                    operation_id: OperationId::new(),
                    payload: RequestPayload::CreateSession(
                        terminal_protocol::CreateSessionRequest {
                            backend: terminal_domain::BackendKind::Native,
                            spec: CreateSessionSpec {
                                title: Some(label.to_string()),
                                ..CreateSessionSpec::default()
                            },
                        },
                    ),
                })
                .await
                .expect("create session should succeed");
            let session_id = match created.payload {
                ResponsePayload::CreateSession(created) => created.session.session_id,
                other => panic!("unexpected payload: {other:?}"),
            };

            daemon
                .handle_request(RequestEnvelope {
                    operation_id: OperationId::new(),
                    payload: RequestPayload::DispatchMuxCommand(
                        terminal_protocol::DispatchMuxCommandRequest {
                            session_id,
                            command: MuxCommand::SaveSession,
                        },
                    ),
                })
                .await
                .expect("save session should succeed");
        }

        let pruned = daemon
            .handle_request(RequestEnvelope {
                operation_id: OperationId::new(),
                payload: RequestPayload::PruneSavedSessions(
                    terminal_protocol::PruneSavedSessionsRequest { keep_latest: 1 },
                ),
            })
            .await
            .expect("prune saved sessions should succeed");
        let list = daemon
            .handle_request(RequestEnvelope {
                operation_id: OperationId::new(),
                payload: RequestPayload::ListSavedSessions,
            })
            .await
            .expect("list saved sessions should succeed");

        match pruned.payload {
            ResponsePayload::PruneSavedSessions(pruned) => {
                assert_eq!(pruned.deleted_count, 2);
                assert_eq!(pruned.kept_count, 1);
            }
            other => panic!("unexpected payload: {other:?}"),
        }

        match list.payload {
            ResponsePayload::ListSavedSessions(listed) => {
                assert_eq!(listed.sessions.len(), 1);
            }
            other => panic!("unexpected payload: {other:?}"),
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn exposes_saved_session_degraded_reason_when_manifest_is_incompatible() {
        let manifest = SavedSessionManifest {
            binary_version: CURRENT_BINARY_VERSION.to_string(),
            protocol_major: CURRENT_PROTOCOL_MAJOR,
            protocol_minor: CURRENT_PROTOCOL_MINOR + 1,
            format_version: 1,
        };
        let (daemon, session_id) = save_incompatible_snapshot("protocol-minor-ahead", manifest);

        let listed = daemon
            .handle_request(RequestEnvelope {
                operation_id: OperationId::new(),
                payload: RequestPayload::ListSavedSessions,
            })
            .await
            .expect("list saved sessions should succeed");
        let saved = daemon
            .handle_request(RequestEnvelope {
                operation_id: OperationId::new(),
                payload: RequestPayload::GetSavedSession(
                    terminal_protocol::GetSavedSessionRequest { session_id },
                ),
            })
            .await
            .expect("get saved session should succeed");
        let restored = daemon
            .handle_request(RequestEnvelope {
                operation_id: OperationId::new(),
                payload: RequestPayload::RestoreSavedSession(
                    terminal_protocol::RestoreSavedSessionRequest { session_id },
                ),
            })
            .await;

        match listed.payload {
            ResponsePayload::ListSavedSessions(listed) => {
                assert_eq!(listed.sessions.len(), 1);
                let session = &listed.sessions[0];
                assert_eq!(session.session_id, session_id);
                assert_eq!(
                    session.compatibility.status,
                    SavedSessionCompatibilityStatus::ProtocolMinorAhead
                );
                assert!(!session.compatibility.can_restore);
            }
            other => panic!("unexpected payload: {other:?}"),
        }

        match saved.payload {
            ResponsePayload::SavedSession(saved) => {
                assert_eq!(saved.session.session_id, session_id);
                assert_eq!(
                    saved.session.compatibility.status,
                    SavedSessionCompatibilityStatus::ProtocolMinorAhead
                );
                assert!(!saved.session.compatibility.can_restore);
            }
            other => panic!("unexpected payload: {other:?}"),
        }

        let error = restored.expect_err("restore should fail for incompatible saved session");
        assert_eq!(error.code, "backend_unsupported");
        assert_eq!(
            error.degraded_reason,
            Some(terminal_domain::DegradedModeReason::SavedSessionIncompatible)
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn routes_topology_screen_and_subscription_requests() {
        let daemon = isolated_daemon();
        let created = daemon
            .handle_request(RequestEnvelope {
                operation_id: OperationId::new(),
                payload: RequestPayload::CreateSession(terminal_protocol::CreateSessionRequest {
                    backend: terminal_domain::BackendKind::Native,
                    spec: CreateSessionSpec {
                        title: Some("screen-shell".to_string()),
                        ..CreateSessionSpec::default()
                    },
                }),
            })
            .await
            .expect("create session should succeed");
        let session_id = match created.payload {
            ResponsePayload::CreateSession(created) => created.session.session_id,
            other => panic!("unexpected payload: {other:?}"),
        };

        let topology = daemon
            .handle_request(RequestEnvelope {
                operation_id: OperationId::new(),
                payload: RequestPayload::GetTopologySnapshot(
                    terminal_protocol::GetTopologySnapshotRequest { session_id },
                ),
            })
            .await
            .expect("topology snapshot should succeed");
        let pane_id = match topology.payload {
            ResponsePayload::TopologySnapshot(snapshot) => {
                snapshot.tabs[0].focused_pane.expect("focused pane should exist")
            }
            other => panic!("unexpected payload: {other:?}"),
        };

        let screen = daemon
            .handle_request(RequestEnvelope {
                operation_id: OperationId::new(),
                payload: RequestPayload::GetScreenSnapshot(
                    terminal_protocol::GetScreenSnapshotRequest { session_id, pane_id },
                ),
            })
            .await
            .expect("screen snapshot should succeed");
        let sequence = match screen.payload {
            ResponsePayload::ScreenSnapshot(snapshot) => snapshot.sequence,
            other => panic!("unexpected payload: {other:?}"),
        };

        let delta = daemon
            .handle_request(RequestEnvelope {
                operation_id: OperationId::new(),
                payload: RequestPayload::GetScreenDelta(terminal_protocol::GetScreenDeltaRequest {
                    session_id,
                    pane_id,
                    from_sequence: sequence,
                }),
            })
            .await
            .expect("screen delta should succeed");
        assert!(matches!(delta.payload, ResponsePayload::ScreenDelta(_)));

        let subscription = daemon
            .open_subscription(terminal_protocol::OpenSubscriptionRequest {
                session_id,
                spec: SubscriptionSpec::SessionTopology,
            })
            .await
            .expect("open subscription should succeed");
        assert!(!subscription.subscription_id.0.as_hyphenated().to_string().is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn routes_dispatch_mux_command_requests() {
        let daemon = isolated_daemon();
        let created = daemon
            .handle_request(RequestEnvelope {
                operation_id: OperationId::new(),
                payload: RequestPayload::CreateSession(terminal_protocol::CreateSessionRequest {
                    backend: terminal_domain::BackendKind::Native,
                    spec: CreateSessionSpec {
                        title: Some("mux-shell".to_string()),
                        ..CreateSessionSpec::default()
                    },
                }),
            })
            .await
            .expect("create session should succeed");
        let session_id = match created.payload {
            ResponsePayload::CreateSession(created) => created.session.session_id,
            other => panic!("unexpected payload: {other:?}"),
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
            .expect("dispatch mux command should succeed");

        match response.payload {
            ResponsePayload::DispatchMuxCommand(result) => {
                assert!(result.changed);
            }
            other => panic!("unexpected payload: {other:?}"),
        }
    }
}
