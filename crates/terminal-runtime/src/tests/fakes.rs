use super::prelude::*;

#[derive(Debug, Default)]
pub(super) struct FakeImportedBackend {
    attached_session_ids: Mutex<Vec<SessionId>>,
}

#[derive(Debug)]
pub(super) struct FakeNativeBackend;

impl MuxBackendPort for FakeNativeBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Native
    }

    fn capabilities(&self) -> BoxFuture<'_, Result<BackendCapabilities, BackendError>> {
        Box::pin(async { Ok(BackendCapabilities::default()) })
    }

    fn discover_sessions(
        &self,
        _scope: terminal_backend_api::BackendScope,
    ) -> BoxFuture<'_, Result<Vec<DiscoveredSession>, BackendError>> {
        Box::pin(async { Ok(Vec::new()) })
    }

    fn create_session(
        &self,
        _spec: CreateSessionSpec,
    ) -> BoxFuture<'_, Result<BackendSessionBinding, BackendError>> {
        Box::pin(async {
            let session_id = SessionId::new();
            Ok(BackendSessionBinding { session_id, route: local_native_route(session_id) })
        })
    }

    fn attach_session(
        &self,
        session_id: SessionId,
        route: SessionRoute,
    ) -> BoxFuture<'_, Result<Box<dyn BackendSessionPort>, BackendError>> {
        Box::pin(async move {
            Ok(Box::new(FakeImportedSession::new(session_id, route)) as Box<dyn BackendSessionPort>)
        })
    }

    fn list_sessions(
        &self,
        _scope: terminal_backend_api::BackendScope,
    ) -> BoxFuture<'_, Result<Vec<BackendSessionSummary>, BackendError>> {
        Box::pin(async { Ok(Vec::new()) })
    }
}

impl FakeImportedBackend {
    pub(super) fn attached_session_ids(&self) -> Vec<SessionId> {
        self.attached_session_ids.lock().expect("attached session ids should lock").clone()
    }
}

impl MuxBackendPort for FakeImportedBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Tmux
    }

    fn capabilities(&self) -> BoxFuture<'_, Result<BackendCapabilities, BackendError>> {
        Box::pin(async { Ok(BackendCapabilities::default()) })
    }

    fn discover_sessions(
        &self,
        _scope: terminal_backend_api::BackendScope,
    ) -> BoxFuture<'_, Result<Vec<DiscoveredSession>, BackendError>> {
        Box::pin(async { Ok(Vec::new()) })
    }

    fn create_session(
        &self,
        _spec: CreateSessionSpec,
    ) -> BoxFuture<'_, Result<BackendSessionBinding, BackendError>> {
        Box::pin(async {
            Err(BackendError::unsupported(
                "foreign backend sessions are imported",
                terminal_domain::DegradedModeReason::ImportedForeignSession,
            ))
        })
    }

    fn attach_session(
        &self,
        session_id: SessionId,
        route: SessionRoute,
    ) -> BoxFuture<'_, Result<Box<dyn BackendSessionPort>, BackendError>> {
        self.attached_session_ids
            .lock()
            .expect("attached session ids should lock")
            .push(session_id);
        Box::pin(async move {
            Ok(Box::new(FakeImportedSession::new(session_id, route)) as Box<dyn BackendSessionPort>)
        })
    }

    fn list_sessions(
        &self,
        _scope: terminal_backend_api::BackendScope,
    ) -> BoxFuture<'_, Result<Vec<BackendSessionSummary>, BackendError>> {
        Box::pin(async { Ok(Vec::new()) })
    }
}

#[derive(Debug)]
struct FakeImportedSession {
    topology: TopologySnapshot,
    screen: ScreenSnapshot,
}

impl FakeImportedSession {
    fn new(session_id: SessionId, route: SessionRoute) -> Self {
        let pane_id = PaneId::new();
        let tab_id = TabId::new();
        let title = route
            .external
            .as_ref()
            .map(|external| external.value.clone())
            .unwrap_or_else(|| "imported".to_string());
        Self {
            topology: TopologySnapshot {
                session_id,
                backend_kind: route.backend,
                tabs: vec![TabSnapshot {
                    tab_id,
                    title: Some(title.clone()),
                    root: PaneTreeNode::Leaf { pane_id },
                    focused_pane: Some(pane_id),
                }],
                focused_tab: Some(tab_id),
            },
            screen: ScreenSnapshot {
                pane_id,
                sequence: 1,
                rows: 24,
                cols: 80,
                source: ProjectionSource::TmuxCapturePane,
                buffer_kind: ScreenBufferKind::Unknown,
                surface: ScreenSurface {
                    title: Some(title),
                    working_directory_uri: None,
                    user_variables: Default::default(),
                    cursor: None,
                    palette: Default::default(),
                    bell_count: 0,
                    progress: Default::default(),
                    lines: Vec::new(),
                },
            },
        }
    }
}

impl BackendSessionPort for FakeImportedSession {
    fn topology_snapshot(&self) -> BoxFuture<'_, Result<TopologySnapshot, BackendError>> {
        let topology = self.topology.clone();
        Box::pin(async move { Ok(topology) })
    }

    fn screen_snapshot(
        &self,
        _pane_id: PaneId,
    ) -> BoxFuture<'_, Result<ScreenSnapshot, BackendError>> {
        let screen = self.screen.clone();
        Box::pin(async move { Ok(screen) })
    }

    fn screen_delta(
        &self,
        _pane_id: PaneId,
        _from_sequence: u64,
    ) -> BoxFuture<'_, Result<ScreenDelta, BackendError>> {
        let screen = self.screen.clone();
        Box::pin(async move { Ok(ScreenDelta::unchanged_from(&screen)) })
    }

    fn dispatch(
        &self,
        _command: MuxCommand,
    ) -> BoxFuture<'_, Result<MuxCommandResult, BackendError>> {
        Box::pin(async { Ok(MuxCommandResult { changed: false }) })
    }

    fn subscribe(
        &self,
        _spec: SubscriptionSpec,
    ) -> BoxFuture<'_, Result<BackendSubscription, BackendError>> {
        let (events_tx, events_rx) = mpsc::channel(1);
        drop(events_tx);
        let (cancel_tx, _cancel_rx) = oneshot::channel();
        Box::pin(async move {
            Ok(BackendSubscription::new(SubscriptionId::new(), events_rx, cancel_tx))
        })
    }
}
