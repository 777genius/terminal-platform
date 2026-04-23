mod backend_catalog;
mod registry;
mod sessions;

use thiserror::Error;

use terminal_backend_api::{
    BackendCapabilities, BackendError, BackendSessionSummary, BackendSubscription,
    CreateSessionSpec, DiscoveredSession, MuxCommand, MuxCommandResult, SubscriptionSpec,
};
use terminal_domain::{
    BackendKind, CURRENT_BINARY_VERSION, CURRENT_PROTOCOL_MAJOR, CURRENT_PROTOCOL_MINOR, PaneId,
    SessionId, SessionRoute,
};
use terminal_persistence::{
    PrunedSavedSessions, SavedNativeSession, SavedSessionSummary, SqliteSessionStore,
};
use terminal_projection::{ScreenDelta, ScreenSnapshot, SessionHealthSnapshot, TopologySnapshot};

pub use backend_catalog::BackendCatalog;
pub use registry::{InMemorySessionRegistry, SessionDescriptor, SessionRegistry};

use sessions::SessionService;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimePhase {
    Starting,
    Ready,
    Degraded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeProtocolVersion {
    pub major: u16,
    pub minor: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeCapabilities {
    pub request_reply: bool,
    pub topology_subscriptions: bool,
    pub pane_subscriptions: bool,
    pub backend_discovery: bool,
    pub backend_capability_queries: bool,
    pub saved_sessions: bool,
    pub session_restore: bool,
    pub degraded_error_reasons: bool,
    pub session_health: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeHandshake {
    pub protocol_version: RuntimeProtocolVersion,
    pub binary_version: String,
    pub daemon_phase: RuntimePhase,
    pub capabilities: RuntimeCapabilities,
    pub available_backends: Vec<BackendKind>,
    pub session_scope: String,
}

pub struct TerminalRuntime {
    sessions: SessionService,
}

impl TerminalRuntime {
    #[must_use]
    pub fn builder() -> TerminalRuntimeBuilder {
        TerminalRuntimeBuilder::default()
    }

    #[must_use]
    pub fn new(backends: BackendCatalog) -> Self {
        Self::builder()
            .with_backends(backends)
            .with_default_persistence()
            .expect("default sqlite session store should open")
            .build()
            .expect("terminal runtime builder should have backends configured")
    }

    #[must_use]
    pub fn with_persistence(backends: BackendCatalog, persistence: SqliteSessionStore) -> Self {
        Self { sessions: SessionService::with_persistence(backends, persistence) }
    }

    #[must_use]
    pub fn handshake(&self) -> RuntimeHandshake {
        RuntimeHandshake {
            protocol_version: RuntimeProtocolVersion {
                major: CURRENT_PROTOCOL_MAJOR,
                minor: CURRENT_PROTOCOL_MINOR,
            },
            binary_version: CURRENT_BINARY_VERSION.to_string(),
            daemon_phase: RuntimePhase::Ready,
            capabilities: RuntimeCapabilities {
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
            available_backends: self.sessions.available_backends(),
            session_scope: "current_user".to_string(),
        }
    }

    #[must_use]
    pub fn session_count(&self) -> usize {
        self.sessions.session_count()
    }

    #[must_use]
    pub fn list_sessions(&self) -> Vec<BackendSessionSummary> {
        self.sessions.list_sessions()
    }

    pub fn list_saved_sessions(&self) -> Result<Vec<SavedSessionSummary>, BackendError> {
        self.sessions.list_saved_sessions()
    }

    pub fn saved_session(&self, session_id: SessionId) -> Result<SavedNativeSession, BackendError> {
        self.sessions.saved_session(session_id)
    }

    pub fn delete_saved_session(&self, session_id: SessionId) -> Result<(), BackendError> {
        self.sessions.delete_saved_session(session_id)
    }

    pub fn prune_saved_sessions(
        &self,
        keep_latest: usize,
    ) -> Result<PrunedSavedSessions, BackendError> {
        self.sessions.prune_saved_sessions(keep_latest)
    }

    pub async fn restore_saved_session(
        &self,
        session_id: SessionId,
    ) -> Result<BackendSessionSummary, BackendError> {
        self.sessions.restore_saved_session(session_id).await
    }

    pub async fn discover_sessions(
        &self,
        backend: BackendKind,
    ) -> Result<Vec<DiscoveredSession>, BackendError> {
        self.sessions.discover_sessions(backend).await
    }

    pub async fn backend_capabilities(
        &self,
        backend: BackendKind,
    ) -> Result<BackendCapabilities, BackendError> {
        self.sessions.backend_capabilities(backend).await
    }

    pub async fn create_session(
        &self,
        backend: BackendKind,
        spec: CreateSessionSpec,
    ) -> Result<BackendSessionSummary, BackendError> {
        self.sessions.create_session(backend, spec).await
    }

    pub async fn import_session(
        &self,
        route: SessionRoute,
        title: Option<String>,
    ) -> Result<BackendSessionSummary, BackendError> {
        self.sessions.import_session(route, title).await
    }

    pub async fn topology_snapshot(
        &self,
        session_id: SessionId,
    ) -> Result<TopologySnapshot, BackendError> {
        self.sessions.topology_snapshot(session_id).await
    }

    pub async fn screen_snapshot(
        &self,
        session_id: SessionId,
        pane_id: PaneId,
    ) -> Result<ScreenSnapshot, BackendError> {
        self.sessions.screen_snapshot(session_id, pane_id).await
    }

    pub async fn screen_delta(
        &self,
        session_id: SessionId,
        pane_id: PaneId,
        from_sequence: u64,
    ) -> Result<ScreenDelta, BackendError> {
        self.sessions.screen_delta(session_id, pane_id, from_sequence).await
    }

    pub async fn dispatch(
        &self,
        session_id: SessionId,
        command: MuxCommand,
    ) -> Result<MuxCommandResult, BackendError> {
        self.sessions.dispatch(session_id, command).await
    }

    pub fn session_health_snapshot(
        &self,
        session_id: SessionId,
    ) -> Result<SessionHealthSnapshot, BackendError> {
        self.sessions.session_health_snapshot(session_id)
    }

    pub async fn open_subscription(
        &self,
        session_id: SessionId,
        spec: SubscriptionSpec,
    ) -> Result<BackendSubscription, BackendError> {
        self.sessions.open_subscription(session_id, spec).await
    }
}

#[derive(Default)]
pub struct TerminalRuntimeBuilder {
    backends: Option<BackendCatalog>,
    persistence: Option<SqliteSessionStore>,
}

impl TerminalRuntimeBuilder {
    #[must_use]
    pub fn with_backends(mut self, backends: BackendCatalog) -> Self {
        self.backends = Some(backends);
        self
    }

    #[must_use]
    pub fn with_persistence(mut self, persistence: SqliteSessionStore) -> Self {
        self.persistence = Some(persistence);
        self
    }

    pub fn with_default_persistence(
        mut self,
    ) -> Result<Self, terminal_persistence::PersistenceError> {
        self.persistence = Some(SqliteSessionStore::open_default()?);
        Ok(self)
    }

    pub fn build(self) -> Result<TerminalRuntime, TerminalRuntimeBuildError> {
        let backends = self.backends.ok_or(TerminalRuntimeBuildError::MissingBackends)?;
        let persistence = self.persistence.ok_or(TerminalRuntimeBuildError::MissingPersistence)?;
        Ok(TerminalRuntime::with_persistence(backends, persistence))
    }
}

#[derive(Debug, Error)]
pub enum TerminalRuntimeBuildError {
    #[error("terminal runtime builder requires a backend catalog")]
    MissingBackends,
    #[error("terminal runtime builder requires a persistence store")]
    MissingPersistence,
}

#[cfg(test)]
mod tests {
    use std::{
        path::{Path, PathBuf},
        sync::{Arc, Mutex},
        time::{SystemTime, UNIX_EPOCH},
    };

    use rusqlite::Connection;
    use terminal_backend_api::{
        BackendCapabilities, BackendError, BackendSessionBinding, BackendSessionPort,
        BackendSessionSummary, BackendSubscription, BoxFuture, CreateSessionSpec,
        DiscoveredSession, MuxBackendPort, MuxCommand, MuxCommandResult, SubscriptionSpec,
    };
    use terminal_backend_native::NativeBackend;
    use terminal_domain::{
        BackendKind, ExternalSessionRef, PaneId, RouteAuthority, SessionId, SessionRoute,
        SubscriptionId, TabId,
    };
    use terminal_mux_domain::{PaneTreeNode, TabSnapshot};
    use terminal_persistence::SqliteSessionStore;
    use terminal_projection::{
        ProjectionSource, ScreenDelta, ScreenSnapshot, ScreenSurface, TopologySnapshot,
    };
    use tokio::sync::{mpsc, oneshot};

    use super::{BackendCatalog, RuntimePhase, TerminalRuntime};

    #[test]
    fn runtime_handshake_reflects_available_backends() {
        let runtime = TerminalRuntime::new(BackendCatalog::new([
            Arc::new(NativeBackend::default()) as Arc<dyn MuxBackendPort>,
        ]));
        let handshake = runtime.handshake();

        assert_eq!(handshake.daemon_phase, RuntimePhase::Ready);
        assert_eq!(handshake.available_backends, vec![terminal_domain::BackendKind::Native]);
        assert_eq!(runtime.session_count(), 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn import_session_reuses_canonical_id_for_same_route_in_one_process() {
        let store = SqliteSessionStore::open(unique_runtime_store_path("import-same-process"))
            .expect("isolated sqlite store should open");
        let backend = Arc::new(FakeImportedBackend::default());
        let runtime = TerminalRuntime::with_persistence(runtime_backends(backend.clone()), store);
        let route = foreign_route("workspace-a");

        let first = runtime
            .import_session(route.clone(), Some("workspace-a".to_string()))
            .await
            .expect("first import should succeed");
        let second = runtime
            .import_session(route, Some("workspace-a".to_string()))
            .await
            .expect("second import should reuse the existing session");

        assert_eq!(first.session_id, second.session_id);
        assert_eq!(backend.attached_session_ids(), vec![first.session_id]);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn import_session_reuses_persisted_canonical_id_after_restart_and_distinguishes_routes() {
        let path = unique_runtime_store_path("import-restart");
        let route_a = foreign_route("workspace-a");
        let route_b = foreign_route("workspace-b");

        let first_session_id = {
            let store = SqliteSessionStore::open(&path).expect("isolated sqlite store should open");
            let backend = Arc::new(FakeImportedBackend::default());
            let runtime = TerminalRuntime::with_persistence(runtime_backends(backend), store);
            runtime
                .import_session(route_a.clone(), Some("workspace-a".to_string()))
                .await
                .expect("first import should succeed")
                .session_id
        };

        let store = SqliteSessionStore::open(&path).expect("reopened sqlite store should open");
        let backend = Arc::new(FakeImportedBackend::default());
        let runtime = TerminalRuntime::with_persistence(runtime_backends(backend), store);
        let repeated = runtime
            .import_session(route_a, Some("workspace-a".to_string()))
            .await
            .expect("reimport after restart should succeed");
        let distinct = runtime
            .import_session(route_b, Some("workspace-b".to_string()))
            .await
            .expect("different foreign route should import separately");

        assert_eq!(repeated.session_id, first_session_id);
        assert_ne!(distinct.session_id, first_session_id);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn import_session_on_legacy_store_creates_route_registry_record() {
        let path = unique_runtime_store_path("legacy-route-registry");
        seed_legacy_saved_session_schema(&path);

        let store = SqliteSessionStore::open(&path).expect("legacy sqlite store should migrate");
        let backend = Arc::new(FakeImportedBackend::default());
        let runtime = TerminalRuntime::with_persistence(runtime_backends(backend), store);
        let route = foreign_route("legacy-import");
        let imported = runtime
            .import_session(route.clone(), Some("legacy-import".to_string()))
            .await
            .expect("legacy import should succeed");

        let reopened =
            SqliteSessionStore::open(&path).expect("migrated sqlite store should reopen");
        let fingerprint = format!(
            "v1/{:?}/{:?}/{}/{}",
            route.backend,
            route.authority,
            route.external.as_ref().expect("foreign route must have external ref").namespace,
            route.external.as_ref().expect("foreign route must have external ref").value,
        );
        let record = reopened
            .load_session_route_by_fingerprint(&fingerprint)
            .expect("route registry lookup should succeed")
            .expect("route registry record should exist");

        assert_eq!(record.session_id, imported.session_id);
        assert_eq!(record.route, route);
    }

    fn runtime_backends(imported_backend: Arc<FakeImportedBackend>) -> BackendCatalog {
        BackendCatalog::new([
            Arc::new(NativeBackend::default()) as Arc<dyn MuxBackendPort>,
            imported_backend as Arc<dyn MuxBackendPort>,
        ])
    }

    fn foreign_route(value: &str) -> SessionRoute {
        SessionRoute {
            backend: BackendKind::Tmux,
            authority: RouteAuthority::ImportedForeign,
            external: Some(ExternalSessionRef {
                namespace: "tmux_session".to_string(),
                value: value.to_string(),
            }),
        }
    }

    fn unique_runtime_store_path(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        std::env::temp_dir()
            .join(format!("terminal-runtime-{label}-{}-{nanos}.sqlite3", std::process::id()))
    }

    fn seed_legacy_saved_session_schema(path: &Path) {
        let connection = Connection::open(path).expect("legacy sqlite file should open");
        connection
            .execute_batch(
                "
                CREATE TABLE native_saved_sessions (
                    session_id TEXT PRIMARY KEY,
                    route_json TEXT NOT NULL,
                    title TEXT,
                    launch_json TEXT,
                    manifest_json TEXT NOT NULL,
                    topology_json TEXT NOT NULL,
                    screens_json TEXT NOT NULL,
                    saved_at_ms INTEGER NOT NULL
                );
                CREATE TABLE __rusqlite_migration_schema_history (
                    version INTEGER PRIMARY KEY,
                    description TEXT NOT NULL,
                    installed_on TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    success INTEGER NOT NULL
                );
                INSERT INTO __rusqlite_migration_schema_history (version, description, success)
                VALUES (0, 'initial native_saved_sessions schema', 1), (1, 'persistence bootstrap noop', 1);
                ",
            )
            .expect("legacy schema should seed");
    }

    #[derive(Debug, Default)]
    struct FakeImportedBackend {
        attached_session_ids: Mutex<Vec<SessionId>>,
    }

    impl FakeImportedBackend {
        fn attached_session_ids(&self) -> Vec<SessionId> {
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
                Ok(Box::new(FakeImportedSession::new(session_id, route))
                    as Box<dyn BackendSessionPort>)
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
                    surface: ScreenSurface { title: Some(title), cursor: None, lines: Vec::new() },
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
}
