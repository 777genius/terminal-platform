use terminal_backend_api::BackendSubscription;
use terminal_protocol::{
    BackendCapabilitiesResponse, CreateSessionResponse, DeleteSavedSessionResponse,
    DiscoverSessionsResponse, ImportSessionResponse, ListSavedSessionsResponse,
    ListSessionsResponse, OpenSubscriptionRequest, OpenSubscriptionResponse, ProtocolError,
    PruneSavedSessionsResponse, RequestEnvelope, RequestPayload, ResponseEnvelope, ResponsePayload,
    RestoreSavedSessionResponse, SavedSessionResponse,
};

use crate::adapters::{
    map_backend_error, map_restore_saved_session_response, map_saved_session_record,
    map_saved_session_summary,
};

use super::runtime_port::TerminalDaemonRuntimePort;

pub struct TerminalDaemonRequestDispatcher<Runtime> {
    runtime: Runtime,
}

impl<Runtime> TerminalDaemonRequestDispatcher<Runtime> {
    #[must_use]
    pub fn new(runtime: Runtime) -> Self {
        Self { runtime }
    }
}

impl<Runtime> TerminalDaemonRequestDispatcher<Runtime>
where
    Runtime: TerminalDaemonRuntimePort,
{
    pub async fn handle_request(
        &self,
        request: RequestEnvelope,
    ) -> Result<ResponseEnvelope, ProtocolError> {
        let payload = match request.payload {
            RequestPayload::Handshake => ResponsePayload::Handshake(self.runtime.handshake()),
            RequestPayload::CreateSession(request) => {
                let session = self
                    .runtime
                    .create_session(request.backend, request.spec)
                    .await
                    .map_err(map_backend_error)?;

                ResponsePayload::CreateSession(CreateSessionResponse { session })
            }
            RequestPayload::ListSessions => ResponsePayload::ListSessions(ListSessionsResponse {
                sessions: self.runtime.list_sessions(),
            }),
            RequestPayload::ListSavedSessions => {
                ResponsePayload::ListSavedSessions(ListSavedSessionsResponse {
                    sessions: self
                        .runtime
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
                        .runtime
                        .discover_sessions(request.backend)
                        .await
                        .map_err(map_backend_error)?,
                })
            }
            RequestPayload::GetBackendCapabilities(request) => {
                ResponsePayload::BackendCapabilities(BackendCapabilitiesResponse {
                    backend: request.backend,
                    capabilities: self
                        .runtime
                        .backend_capabilities(request.backend)
                        .await
                        .map_err(map_backend_error)?,
                })
            }
            RequestPayload::ImportSession(request) => {
                let session = self
                    .runtime
                    .import_session(request.route, request.title)
                    .await
                    .map_err(map_backend_error)?;

                ResponsePayload::ImportSession(ImportSessionResponse { session })
            }
            RequestPayload::GetSavedSession(request) => {
                ResponsePayload::SavedSession(SavedSessionResponse {
                    session: map_saved_session_record(
                        self.runtime
                            .saved_session(request.session_id)
                            .map_err(map_backend_error)?,
                    ),
                })
            }
            RequestPayload::DeleteSavedSession(request) => {
                self.runtime.delete_saved_session(request.session_id).map_err(map_backend_error)?;
                ResponsePayload::DeleteSavedSession(DeleteSavedSessionResponse {
                    session_id: request.session_id,
                })
            }
            RequestPayload::PruneSavedSessions(request) => {
                let pruned = self
                    .runtime
                    .prune_saved_sessions(request.keep_latest)
                    .map_err(map_backend_error)?;
                ResponsePayload::PruneSavedSessions(PruneSavedSessionsResponse {
                    deleted_count: pruned.deleted_count,
                    kept_count: pruned.kept_count,
                })
            }
            RequestPayload::RestoreSavedSession(request) => {
                let saved =
                    self.runtime.saved_session(request.session_id).map_err(map_backend_error)?;
                ResponsePayload::RestoreSavedSession(RestoreSavedSessionResponse {
                    ..map_restore_saved_session_response(
                        request.session_id,
                        &saved,
                        self.runtime
                            .restore_saved_session(request.session_id)
                            .await
                            .map_err(map_backend_error)?,
                    )
                })
            }
            RequestPayload::GetTopologySnapshot(request) => ResponsePayload::TopologySnapshot(
                self.runtime
                    .topology_snapshot(request.session_id)
                    .await
                    .map_err(map_backend_error)?,
            ),
            RequestPayload::GetScreenSnapshot(request) => ResponsePayload::ScreenSnapshot(
                self.runtime
                    .screen_snapshot(request.session_id, request.pane_id)
                    .await
                    .map_err(map_backend_error)?,
            ),
            RequestPayload::GetScreenDelta(request) => ResponsePayload::ScreenDelta(
                self.runtime
                    .screen_delta(request.session_id, request.pane_id, request.from_sequence)
                    .await
                    .map_err(map_backend_error)?,
            ),
            RequestPayload::DispatchMuxCommand(request) => ResponsePayload::DispatchMuxCommand(
                self.runtime
                    .dispatch(request.session_id, request.command)
                    .await
                    .map_err(map_backend_error)?,
            ),
            RequestPayload::OpenSubscription(request) => {
                ResponsePayload::SubscriptionOpened(OpenSubscriptionResponse {
                    subscription_id: self
                        .runtime
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
    ) -> Result<BackendSubscription, ProtocolError> {
        self.runtime
            .open_subscription(request.session_id, request.spec)
            .await
            .map_err(map_backend_error)
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
        ProjectionSource, ScreenDelta, ScreenSnapshot, ScreenSurface, TopologySnapshot,
    };
    use terminal_protocol::{
        CreateSessionRequest, DaemonCapabilities, DaemonPhase, Handshake, ProtocolVersion,
        RequestEnvelope, RequestPayload, ResponsePayload,
    };

    use crate::application::{
        RuntimePrunedSavedSessions, RuntimeSavedSessionRecord, RuntimeSavedSessionSummary,
    };

    use super::{TerminalDaemonRequestDispatcher, TerminalDaemonRuntimePort};

    struct StubRuntime;

    impl TerminalDaemonRuntimePort for StubRuntime {
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
                },
                available_backends: vec![BackendKind::Native],
                session_scope: "current_user".to_string(),
            }
        }

        fn list_sessions(&self) -> Vec<BackendSessionSummary> {
            Vec::new()
        }

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

        async fn restore_saved_session(
            &self,
            _session_id: SessionId,
        ) -> Result<BackendSessionSummary, BackendError> {
            Err(BackendError::unsupported(
                "restore not exercised in this unit test",
                DegradedModeReason::SavedSessionIncompatible,
            ))
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
        let dispatcher = TerminalDaemonRequestDispatcher::new(StubRuntime);
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
        let dispatcher = TerminalDaemonRequestDispatcher::new(StubRuntime);
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
}
