use terminal_protocol::{
    BackendCapabilitiesResponse, CreateSessionResponse, DeleteSavedSessionResponse,
    DiscoverSessionsResponse, ImportSessionResponse, ListSavedSessionsResponse,
    ListSessionsResponse, ProtocolError, PruneSavedSessionsResponse, RequestEnvelope,
    RequestPayload, ResponseEnvelope, ResponsePayload, RestoreSavedSessionResponse,
    SavedSessionResponse,
};

use crate::{
    adapters::{
        map_backend_error, map_restore_saved_session_response, map_saved_session_record,
        map_saved_session_summary,
    },
    application::{
        TerminalDaemonActiveSessionPort, TerminalDaemonCatalogPort,
        TerminalDaemonSavedSessionsPort, TerminalDaemonSubscriptionPort,
        TerminalDaemonSubscriptionService,
    },
};

pub struct TerminalDaemonRequestDispatcher<Catalog, SavedSessions, ActiveSessions, Subscriptions> {
    catalog: Catalog,
    saved_sessions: SavedSessions,
    active_sessions: ActiveSessions,
    subscriptions: TerminalDaemonSubscriptionService<Subscriptions>,
}

impl<Catalog, SavedSessions, ActiveSessions, Subscriptions>
    TerminalDaemonRequestDispatcher<Catalog, SavedSessions, ActiveSessions, Subscriptions>
{
    #[must_use]
    pub fn new(
        catalog: Catalog,
        saved_sessions: SavedSessions,
        active_sessions: ActiveSessions,
        subscriptions: TerminalDaemonSubscriptionService<Subscriptions>,
    ) -> Self {
        Self { catalog, saved_sessions, active_sessions, subscriptions }
    }
}

impl<Catalog, SavedSessions, ActiveSessions, Subscriptions>
    TerminalDaemonRequestDispatcher<Catalog, SavedSessions, ActiveSessions, Subscriptions>
where
    Catalog: TerminalDaemonCatalogPort,
    SavedSessions: TerminalDaemonSavedSessionsPort,
    ActiveSessions: TerminalDaemonActiveSessionPort,
    Subscriptions: TerminalDaemonSubscriptionPort,
{
    pub async fn handle_request(
        &self,
        request: RequestEnvelope,
    ) -> Result<ResponseEnvelope, ProtocolError> {
        let payload = match request.payload {
            RequestPayload::Handshake => ResponsePayload::Handshake(self.catalog.handshake()),
            RequestPayload::CreateSession(request) => {
                let session = self
                    .catalog
                    .create_session(request.backend, request.spec)
                    .await
                    .map_err(map_backend_error)?;

                ResponsePayload::CreateSession(CreateSessionResponse { session })
            }
            RequestPayload::ListSessions => ResponsePayload::ListSessions(ListSessionsResponse {
                sessions: self.catalog.list_sessions(),
            }),
            RequestPayload::ListSavedSessions => {
                ResponsePayload::ListSavedSessions(ListSavedSessionsResponse {
                    sessions: self
                        .saved_sessions
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
                        .catalog
                        .discover_sessions(request.backend)
                        .await
                        .map_err(map_backend_error)?,
                })
            }
            RequestPayload::GetBackendCapabilities(request) => {
                ResponsePayload::BackendCapabilities(BackendCapabilitiesResponse {
                    backend: request.backend,
                    capabilities: self
                        .catalog
                        .backend_capabilities(request.backend)
                        .await
                        .map_err(map_backend_error)?,
                })
            }
            RequestPayload::ImportSession(request) => {
                let session = self
                    .catalog
                    .import_session(request.route, request.title)
                    .await
                    .map_err(map_backend_error)?;

                ResponsePayload::ImportSession(ImportSessionResponse { session })
            }
            RequestPayload::GetSavedSession(request) => {
                ResponsePayload::SavedSession(SavedSessionResponse {
                    session: map_saved_session_record(
                        self.saved_sessions
                            .saved_session(request.session_id)
                            .map_err(map_backend_error)?,
                    ),
                })
            }
            RequestPayload::DeleteSavedSession(request) => {
                self.saved_sessions
                    .delete_saved_session(request.session_id)
                    .map_err(map_backend_error)?;
                ResponsePayload::DeleteSavedSession(DeleteSavedSessionResponse {
                    session_id: request.session_id,
                })
            }
            RequestPayload::PruneSavedSessions(request) => {
                let pruned = self
                    .saved_sessions
                    .prune_saved_sessions(request.keep_latest)
                    .map_err(map_backend_error)?;
                ResponsePayload::PruneSavedSessions(PruneSavedSessionsResponse {
                    deleted_count: pruned.deleted_count,
                    kept_count: pruned.kept_count,
                })
            }
            RequestPayload::RestoreSavedSession(request) => {
                let saved = self
                    .saved_sessions
                    .saved_session(request.session_id)
                    .map_err(map_backend_error)?;
                ResponsePayload::RestoreSavedSession(RestoreSavedSessionResponse {
                    ..map_restore_saved_session_response(
                        request.session_id,
                        &saved,
                        self.saved_sessions
                            .restore_saved_session(request.session_id)
                            .await
                            .map_err(map_backend_error)?,
                    )
                })
            }
            RequestPayload::GetSessionHealthSnapshot(request) => {
                ResponsePayload::SessionHealthSnapshot(
                    self.active_sessions
                        .session_health_snapshot(request.session_id)
                        .map_err(map_backend_error)?,
                )
            }
            RequestPayload::GetTopologySnapshot(request) => ResponsePayload::TopologySnapshot(
                self.active_sessions
                    .topology_snapshot(request.session_id)
                    .await
                    .map_err(map_backend_error)?,
            ),
            RequestPayload::GetScreenSnapshot(request) => ResponsePayload::ScreenSnapshot(
                self.active_sessions
                    .screen_snapshot(request.session_id, request.pane_id)
                    .await
                    .map_err(map_backend_error)?,
            ),
            RequestPayload::GetScreenDelta(request) => ResponsePayload::ScreenDelta(
                self.active_sessions
                    .screen_delta(request.session_id, request.pane_id, request.from_sequence)
                    .await
                    .map_err(map_backend_error)?,
            ),
            RequestPayload::DispatchMuxCommand(request) => ResponsePayload::DispatchMuxCommand(
                self.active_sessions
                    .dispatch(request.session_id, request.command)
                    .await
                    .map_err(map_backend_error)?,
            ),
            RequestPayload::OpenSubscription(request) => ResponsePayload::SubscriptionOpened(
                self.subscriptions.open_subscription_response(request).await?,
            ),
        };

        Ok(ResponseEnvelope { operation_id: request.operation_id, payload })
    }
}

#[cfg(test)]
mod tests {
    use terminal_backend_api::{
        BackendCapabilities, BackendError, BackendSessionSummary, BackendSubscription,
        CreateSessionSpec, DiscoveredSession, MuxCommand, MuxCommandResult, SubscriptionSpec,
    };
    use terminal_domain::{
        BackendKind, CURRENT_BINARY_VERSION, CURRENT_PROTOCOL_MAJOR, CURRENT_PROTOCOL_MINOR,
        DegradedModeReason, OperationId, PaneId, RouteAuthority, SessionId, SessionRoute,
    };
    use terminal_projection::{
        ProjectionSource, ScreenDelta, ScreenSnapshot, ScreenSurface, SessionHealthSnapshot,
        TopologySnapshot,
    };
    use terminal_protocol::{
        CreateSessionRequest, DaemonCapabilities, DaemonPhase, GetSessionHealthSnapshotRequest,
        Handshake, OpenSubscriptionRequest, ProtocolVersion, RequestEnvelope, RequestPayload,
        ResponsePayload,
    };

    use crate::application::{
        RuntimePrunedSavedSessions, RuntimeSavedSessionRecord, RuntimeSavedSessionSummary,
        TerminalDaemonActiveSessionPort, TerminalDaemonCatalogPort,
        TerminalDaemonSavedSessionsPort, TerminalDaemonSubscriptionPort,
        TerminalDaemonSubscriptionService,
    };

    use super::TerminalDaemonRequestDispatcher;

    struct StubRuntime;

    impl TerminalDaemonCatalogPort for StubRuntime {
        fn handshake(&self) -> Handshake {
            Handshake {
                protocol_version: ProtocolVersion {
                    major: CURRENT_PROTOCOL_MAJOR,
                    minor: CURRENT_PROTOCOL_MINOR,
                },
                binary_version: CURRENT_BINARY_VERSION.to_string(),
                daemon_phase: DaemonPhase::Ready,
                capabilities: DaemonCapabilities {
                    request_reply: true,
                    topology_subscriptions: true,
                    pane_subscriptions: true,
                    backend_discovery: true,
                    backend_capability_queries: true,
                    saved_sessions: true,
                    session_restore: true,
                    degraded_error_reasons: true,
                    session_health: true,
                },
                available_backends: vec![BackendKind::Native],
                session_scope: "current_user".to_string(),
            }
        }

        fn list_sessions(&self) -> Vec<BackendSessionSummary> {
            Vec::new()
        }

        async fn create_session(
            &self,
            backend: BackendKind,
            spec: CreateSessionSpec,
        ) -> Result<BackendSessionSummary, BackendError> {
            Ok(BackendSessionSummary {
                session_id: SessionId::new(),
                route: SessionRoute {
                    backend,
                    authority: RouteAuthority::LocalDaemon,
                    external: None,
                },
                title: spec.title,
            })
        }

        async fn discover_sessions(
            &self,
            _backend: BackendKind,
        ) -> Result<Vec<DiscoveredSession>, BackendError> {
            Ok(Vec::new())
        }

        async fn backend_capabilities(
            &self,
            _backend: BackendKind,
        ) -> Result<BackendCapabilities, BackendError> {
            Ok(BackendCapabilities::default())
        }

        async fn import_session(
            &self,
            _route: SessionRoute,
            title: Option<String>,
        ) -> Result<BackendSessionSummary, BackendError> {
            Ok(BackendSessionSummary {
                session_id: SessionId::new(),
                route: SessionRoute {
                    backend: BackendKind::Tmux,
                    authority: RouteAuthority::ImportedForeign,
                    external: None,
                },
                title,
            })
        }
    }

    impl TerminalDaemonSavedSessionsPort for StubRuntime {
        fn list_saved_sessions(&self) -> Result<Vec<RuntimeSavedSessionSummary>, BackendError> {
            Ok(Vec::new())
        }

        fn saved_session(
            &self,
            _session_id: SessionId,
        ) -> Result<RuntimeSavedSessionRecord, BackendError> {
            Err(BackendError::not_found("missing saved session"))
        }

        fn delete_saved_session(&self, _session_id: SessionId) -> Result<(), BackendError> {
            Ok(())
        }

        fn prune_saved_sessions(
            &self,
            _keep_latest: usize,
        ) -> Result<RuntimePrunedSavedSessions, BackendError> {
            Ok(RuntimePrunedSavedSessions { deleted_count: 0, kept_count: 0 })
        }

        async fn restore_saved_session(
            &self,
            _session_id: SessionId,
        ) -> Result<BackendSessionSummary, BackendError> {
            Err(BackendError::unsupported(
                "restore not exercised in this unit test",
                DegradedModeReason::SavedSessionIncompatible,
            ))
        }
    }

    impl TerminalDaemonActiveSessionPort for StubRuntime {
        fn session_health_snapshot(
            &self,
            session_id: SessionId,
        ) -> Result<SessionHealthSnapshot, BackendError> {
            Ok(SessionHealthSnapshot::ready(session_id))
        }

        async fn topology_snapshot(
            &self,
            _session_id: SessionId,
        ) -> Result<TopologySnapshot, BackendError> {
            Err(BackendError::not_found("topology not exercised in this unit test"))
        }

        async fn screen_snapshot(
            &self,
            _session_id: SessionId,
            _pane_id: PaneId,
        ) -> Result<ScreenSnapshot, BackendError> {
            Ok(ScreenSnapshot {
                pane_id: PaneId::new(),
                sequence: 0,
                rows: 1,
                cols: 1,
                source: ProjectionSource::NativeEmulator,
                surface: ScreenSurface { title: None, cursor: None, lines: Vec::new() },
            })
        }

        async fn screen_delta(
            &self,
            _session_id: SessionId,
            _pane_id: PaneId,
            _from_sequence: u64,
        ) -> Result<ScreenDelta, BackendError> {
            Err(BackendError::not_found("screen delta not exercised in this unit test"))
        }

        async fn dispatch(
            &self,
            _session_id: SessionId,
            _command: MuxCommand,
        ) -> Result<MuxCommandResult, BackendError> {
            Ok(MuxCommandResult { changed: false })
        }
    }

    impl TerminalDaemonSubscriptionPort for StubRuntime {
        async fn open_subscription(
            &self,
            _session_id: SessionId,
            _spec: SubscriptionSpec,
        ) -> Result<BackendSubscription, BackendError> {
            Err(BackendError::not_found("subscriptions not exercised in this unit test"))
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn routes_handshake_through_runtime_port() {
        let dispatcher = TerminalDaemonRequestDispatcher::new(
            StubRuntime,
            StubRuntime,
            StubRuntime,
            TerminalDaemonSubscriptionService::new(StubRuntime),
        );
        let response = dispatcher
            .handle_request(RequestEnvelope {
                operation_id: OperationId::new(),
                payload: RequestPayload::Handshake,
            })
            .await
            .expect("handshake should dispatch through runtime port");

        match response.payload {
            ResponsePayload::Handshake(handshake) => {
                assert_eq!(handshake.daemon_phase, DaemonPhase::Ready);
                assert_eq!(handshake.available_backends, vec![BackendKind::Native]);
            }
            other => panic!("unexpected response payload: {other:?}"),
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn routes_create_session_through_runtime_port() {
        let dispatcher = TerminalDaemonRequestDispatcher::new(
            StubRuntime,
            StubRuntime,
            StubRuntime,
            TerminalDaemonSubscriptionService::new(StubRuntime),
        );
        let response = dispatcher
            .handle_request(RequestEnvelope {
                operation_id: OperationId::new(),
                payload: RequestPayload::CreateSession(CreateSessionRequest {
                    backend: BackendKind::Native,
                    spec: CreateSessionSpec { title: Some("shell".to_string()), launch: None },
                }),
            })
            .await
            .expect("create session should dispatch through runtime port");

        match response.payload {
            ResponsePayload::CreateSession(created) => {
                assert_eq!(created.session.route.backend, BackendKind::Native);
                assert_eq!(created.session.route.authority, RouteAuthority::LocalDaemon);
                assert_eq!(created.session.title.as_deref(), Some("shell"));
            }
            other => panic!("unexpected response payload: {other:?}"),
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn routes_subscription_open_response_through_subscription_service() {
        let dispatcher = TerminalDaemonRequestDispatcher::new(
            StubRuntime,
            StubRuntime,
            StubRuntime,
            TerminalDaemonSubscriptionService::new(StubRuntime),
        );
        let result = dispatcher
            .handle_request(RequestEnvelope {
                operation_id: OperationId::new(),
                payload: RequestPayload::OpenSubscription(OpenSubscriptionRequest {
                    session_id: SessionId::new(),
                    spec: SubscriptionSpec::SessionTopology,
                }),
            })
            .await;

        let error = result.expect_err("stub subscription path should surface backend error");
        assert_eq!(error.code, "backend_not_found");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn routes_session_health_snapshot_through_runtime_port() {
        let dispatcher = TerminalDaemonRequestDispatcher::new(
            StubRuntime,
            StubRuntime,
            StubRuntime,
            TerminalDaemonSubscriptionService::new(StubRuntime),
        );
        let session_id = SessionId::new();
        let response = dispatcher
            .handle_request(RequestEnvelope {
                operation_id: OperationId::new(),
                payload: RequestPayload::GetSessionHealthSnapshot(
                    GetSessionHealthSnapshotRequest { session_id },
                ),
            })
            .await
            .expect("session health should dispatch through runtime port");

        match response.payload {
            ResponsePayload::SessionHealthSnapshot(health) => {
                assert_eq!(health.session_id, session_id);
            }
            other => panic!("unexpected response payload: {other:?}"),
        }
    }
}
