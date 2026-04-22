use terminal_backend_api::{CreateSessionSpec, MuxCommand, MuxCommandResult, SubscriptionSpec};
use terminal_domain::{
    BackendKind, ProtocolCompatibility, ProtocolCompatibilityStatus, SessionRoute,
    protocol_compatibility,
};
use terminal_transport::{LocalSocketTransportClient, LocalSocketTransportSubscription};

use terminal_projection::{
    ScreenDelta, ScreenSnapshot, SessionHealthSnapshot, TopologySnapshot,
};
use terminal_protocol::{
    BackendCapabilitiesResponse, CreateSessionRequest, CreateSessionResponse,
    DeleteSavedSessionRequest, DeleteSavedSessionResponse, DiscoverSessionsRequest,
    DiscoverSessionsResponse, DispatchMuxCommandRequest, GetBackendCapabilitiesRequest,
    GetSavedSessionRequest, GetScreenDeltaRequest, GetScreenSnapshotRequest,
    GetSessionHealthSnapshotRequest, GetTopologySnapshotRequest, Handshake, ImportSessionRequest,
    ImportSessionResponse, ListSavedSessionsResponse, ListSessionsResponse, LocalSocketAddress,
    OpenSubscriptionRequest, ProtocolError, ProtocolVersion, PruneSavedSessionsRequest,
    PruneSavedSessionsResponse, RequestPayload, ResponsePayload, RestoreSavedSessionRequest,
    RestoreSavedSessionResponse, SavedSessionResponse, SubscriptionEvent,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonClientInfo {
    pub expected_protocol: ProtocolVersion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandshakeAssessmentStatus {
    Ready,
    Starting,
    Degraded,
    ProtocolMajorUnsupported,
    ProtocolMinorAhead,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandshakeAssessment {
    pub can_use: bool,
    pub protocol: ProtocolCompatibility,
    pub status: HandshakeAssessmentStatus,
}

impl Default for DaemonClientInfo {
    fn default() -> Self {
        Self { expected_protocol: ProtocolVersion { major: 0, minor: 2 } }
    }
}

impl DaemonClientInfo {
    #[must_use]
    pub fn assess_handshake(&self, handshake: &Handshake) -> HandshakeAssessment {
        let protocol = protocol_compatibility(
            self.expected_protocol.major,
            self.expected_protocol.minor,
            handshake.protocol_version.major,
            handshake.protocol_version.minor,
        );
        let status = match protocol.status {
            ProtocolCompatibilityStatus::Compatible => match handshake.daemon_phase {
                terminal_protocol::DaemonPhase::Ready => HandshakeAssessmentStatus::Ready,
                terminal_protocol::DaemonPhase::Starting => HandshakeAssessmentStatus::Starting,
                terminal_protocol::DaemonPhase::Degraded => HandshakeAssessmentStatus::Degraded,
            },
            ProtocolCompatibilityStatus::ProtocolMajorUnsupported => {
                HandshakeAssessmentStatus::ProtocolMajorUnsupported
            }
            ProtocolCompatibilityStatus::ProtocolMinorAhead => {
                HandshakeAssessmentStatus::ProtocolMinorAhead
            }
        };

        HandshakeAssessment {
            can_use: protocol.can_connect
                && matches!(handshake.daemon_phase, terminal_protocol::DaemonPhase::Ready),
            protocol,
            status,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalSocketDaemonClient {
    transport: LocalSocketTransportClient,
    info: DaemonClientInfo,
}

pub struct LocalSocketSubscription {
    inner: LocalSocketTransportSubscription,
}

impl LocalSocketSubscription {
    #[must_use]
    pub fn subscription_id(&self) -> terminal_domain::SubscriptionId {
        self.inner.subscription_id()
    }

    pub async fn recv(&mut self) -> Result<Option<SubscriptionEvent>, ProtocolError> {
        self.inner.recv().await
    }

    pub async fn close(&mut self) -> Result<(), ProtocolError> {
        self.inner.close().await
    }
}

impl LocalSocketDaemonClient {
    #[must_use]
    pub fn new(address: LocalSocketAddress) -> Self {
        Self {
            transport: LocalSocketTransportClient::new(address),
            info: DaemonClientInfo::default(),
        }
    }

    #[must_use]
    pub fn address(&self) -> &LocalSocketAddress {
        self.transport.address()
    }

    #[must_use]
    pub fn info(&self) -> &DaemonClientInfo {
        &self.info
    }

    pub async fn handshake(&self) -> Result<Handshake, ProtocolError> {
        let response = self.send_request(RequestPayload::Handshake).await?;

        match response.payload {
            ResponsePayload::Handshake(handshake) => Ok(handshake),
            other => Err(ProtocolError::unexpected_payload("handshake", &other)),
        }
    }

    pub async fn handshake_assessment(&self) -> Result<HandshakeAssessment, ProtocolError> {
        let handshake = self.handshake().await?;
        Ok(self.info.assess_handshake(&handshake))
    }

    pub async fn list_sessions(&self) -> Result<ListSessionsResponse, ProtocolError> {
        let response = self.send_request(RequestPayload::ListSessions).await?;

        match response.payload {
            ResponsePayload::ListSessions(list) => Ok(list),
            other => Err(ProtocolError::unexpected_payload("list_sessions", &other)),
        }
    }

    pub async fn list_saved_sessions(&self) -> Result<ListSavedSessionsResponse, ProtocolError> {
        let response = self.send_request(RequestPayload::ListSavedSessions).await?;

        match response.payload {
            ResponsePayload::ListSavedSessions(list) => Ok(list),
            other => Err(ProtocolError::unexpected_payload("list_saved_sessions", &other)),
        }
    }

    pub async fn discover_sessions(
        &self,
        backend: BackendKind,
    ) -> Result<DiscoverSessionsResponse, ProtocolError> {
        let response = self
            .send_request(RequestPayload::DiscoverSessions(DiscoverSessionsRequest { backend }))
            .await?;

        match response.payload {
            ResponsePayload::DiscoverSessions(discovered) => Ok(discovered),
            other => Err(ProtocolError::unexpected_payload("discover_sessions", &other)),
        }
    }

    pub async fn backend_capabilities(
        &self,
        backend: BackendKind,
    ) -> Result<BackendCapabilitiesResponse, ProtocolError> {
        let response = self
            .send_request(RequestPayload::GetBackendCapabilities(GetBackendCapabilitiesRequest {
                backend,
            }))
            .await?;

        match response.payload {
            ResponsePayload::BackendCapabilities(capabilities) => Ok(capabilities),
            other => Err(ProtocolError::unexpected_payload("backend_capabilities", &other)),
        }
    }

    pub async fn create_session(
        &self,
        backend: BackendKind,
        spec: CreateSessionSpec,
    ) -> Result<CreateSessionResponse, ProtocolError> {
        let response = self
            .send_request(RequestPayload::CreateSession(CreateSessionRequest { backend, spec }))
            .await?;

        match response.payload {
            ResponsePayload::CreateSession(created) => Ok(created),
            other => Err(ProtocolError::unexpected_payload("create_session", &other)),
        }
    }

    pub async fn import_session(
        &self,
        route: SessionRoute,
        title: Option<String>,
    ) -> Result<ImportSessionResponse, ProtocolError> {
        let response = self
            .send_request(RequestPayload::ImportSession(ImportSessionRequest { route, title }))
            .await?;

        match response.payload {
            ResponsePayload::ImportSession(imported) => Ok(imported),
            other => Err(ProtocolError::unexpected_payload("import_session", &other)),
        }
    }

    pub async fn saved_session(
        &self,
        session_id: terminal_domain::SessionId,
    ) -> Result<SavedSessionResponse, ProtocolError> {
        let response = self
            .send_request(RequestPayload::GetSavedSession(GetSavedSessionRequest { session_id }))
            .await?;

        match response.payload {
            ResponsePayload::SavedSession(saved) => Ok(saved),
            other => Err(ProtocolError::unexpected_payload("saved_session", &other)),
        }
    }

    pub async fn session_health_snapshot(
        &self,
        session_id: terminal_domain::SessionId,
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

    pub async fn delete_saved_session(
        &self,
        session_id: terminal_domain::SessionId,
    ) -> Result<DeleteSavedSessionResponse, ProtocolError> {
        let response = self
            .send_request(RequestPayload::DeleteSavedSession(DeleteSavedSessionRequest {
                session_id,
            }))
            .await?;

        match response.payload {
            ResponsePayload::DeleteSavedSession(deleted) => Ok(deleted),
            other => Err(ProtocolError::unexpected_payload("delete_saved_session", &other)),
        }
    }

    pub async fn prune_saved_sessions(
        &self,
        keep_latest: usize,
    ) -> Result<PruneSavedSessionsResponse, ProtocolError> {
        let response = self
            .send_request(RequestPayload::PruneSavedSessions(PruneSavedSessionsRequest {
                keep_latest,
            }))
            .await?;

        match response.payload {
            ResponsePayload::PruneSavedSessions(pruned) => Ok(pruned),
            other => Err(ProtocolError::unexpected_payload("prune_saved_sessions", &other)),
        }
    }

    pub async fn restore_saved_session(
        &self,
        session_id: terminal_domain::SessionId,
    ) -> Result<RestoreSavedSessionResponse, ProtocolError> {
        let response = self
            .send_request(RequestPayload::RestoreSavedSession(RestoreSavedSessionRequest {
                session_id,
            }))
            .await?;

        match response.payload {
            ResponsePayload::RestoreSavedSession(restored) => Ok(restored),
            other => Err(ProtocolError::unexpected_payload("restore_saved_session", &other)),
        }
    }

    pub async fn topology_snapshot(
        &self,
        session_id: terminal_domain::SessionId,
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
        session_id: terminal_domain::SessionId,
        pane_id: terminal_domain::PaneId,
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
        session_id: terminal_domain::SessionId,
        pane_id: terminal_domain::PaneId,
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

    pub async fn dispatch(
        &self,
        session_id: terminal_domain::SessionId,
        command: MuxCommand,
    ) -> Result<MuxCommandResult, ProtocolError> {
        let response = self
            .send_request(RequestPayload::DispatchMuxCommand(DispatchMuxCommandRequest {
                session_id,
                command,
            }))
            .await?;

        match response.payload {
            ResponsePayload::DispatchMuxCommand(result) => Ok(result),
            other => Err(ProtocolError::unexpected_payload("dispatch_mux_command", &other)),
        }
    }

    pub async fn open_subscription(
        &self,
        session_id: terminal_domain::SessionId,
        spec: SubscriptionSpec,
    ) -> Result<LocalSocketSubscription, ProtocolError> {
        self.transport
            .open_subscription(OpenSubscriptionRequest { session_id, spec })
            .await
            .map(|inner| LocalSocketSubscription { inner })
    }

    async fn send_request(
        &self,
        payload: RequestPayload,
    ) -> Result<terminal_protocol::ResponseEnvelope, ProtocolError> {
        self.transport.send_request(payload).await
    }
}

#[cfg(test)]
mod tests {
    use std::{
        thread,
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

    use rusqlite::{Connection, params};
    use terminal_backend_api::{
        CreateSessionSpec, MuxCommand, NewTabSpec, SendInputSpec, ShellLaunchSpec, SubscriptionSpec,
    };
    use terminal_daemon::{TerminalDaemon, spawn_local_socket_server};
    use terminal_domain::{
        BackendKind, CURRENT_BINARY_VERSION, CURRENT_PROTOCOL_MAJOR, CURRENT_PROTOCOL_MINOR,
        CURRENT_SAVED_SESSION_FORMAT_VERSION, DegradedModeReason, PaneId,
        SavedSessionCompatibilityStatus, SavedSessionManifest, SessionId, TabId,
        local_native_route,
    };
    use terminal_mux_domain::{PaneTreeNode, TabSnapshot};
    use terminal_persistence::SqliteSessionStore;
    use terminal_projection::TopologySnapshot;
    use terminal_protocol::{
        DaemonCapabilities, DaemonPhase, Handshake, ProtocolVersion, SubscriptionEvent,
    };
    use tokio::time::{sleep, timeout};

    use super::{HandshakeAssessmentStatus, LocalSocketDaemonClient};

    fn unique_address(label: &str) -> terminal_protocol::LocalSocketAddress {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let slug = format!("terminal-platform-{label}-{}-{nanos}.sock", std::process::id());

        terminal_protocol::LocalSocketAddress::from_runtime_slug(slug)
    }

    fn spawn_default_daemon_with_retry(
        address: terminal_protocol::LocalSocketAddress,
    ) -> std::io::Result<terminal_daemon::LocalSocketServerHandle> {
        let attempts = if cfg!(windows) { 50 } else { 5 };
        let retryable_kinds = [
            std::io::ErrorKind::AlreadyExists,
            std::io::ErrorKind::PermissionDenied,
            std::io::ErrorKind::AddrInUse,
        ];
        let mut last_error = None;

        for attempt in 0..attempts {
            match spawn_local_socket_server(TerminalDaemon::default(), address.clone()) {
                Ok(server) => return Ok(server),
                Err(error) if retryable_kinds.contains(&error.kind()) && attempt + 1 < attempts => {
                    last_error = Some(error);
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(error) => return Err(error),
            }
        }

        Err(last_error.unwrap_or_else(|| std::io::Error::other("daemon never rebound on address")))
    }

    fn isolated_daemon() -> TerminalDaemon {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let store = SqliteSessionStore::open(std::env::temp_dir().join(format!(
            "terminal-platform-daemon-client-{}-{nanos}.sqlite3",
            std::process::id()
        )))
        .expect("isolated sqlite session store should open");

        TerminalDaemon::with_persistence(store)
    }

    fn isolated_daemon_with_saved_snapshot(
        label: &str,
        manifest: SavedSessionManifest,
    ) -> (TerminalDaemon, SessionId) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let path = std::env::temp_dir().join(format!(
            "terminal-platform-daemon-client-{label}-{}-{nanos}.sqlite3",
            std::process::id()
        ));
        let store =
            SqliteSessionStore::open(&path).expect("isolated sqlite session store should open");
        let session_id = SessionId::new();
        let tab_id = TabId::new();
        let pane_id = PaneId::new();
        store
            .save_native_session(&terminal_persistence::SavedNativeSession {
                session_id,
                route: local_native_route(session_id),
                title: Some("future-shell".to_string()),
                launch: None,
                manifest,
                topology: TopologySnapshot {
                    session_id,
                    backend_kind: BackendKind::Native,
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

    fn isolated_daemon_with_valid_and_corrupted_saved_rows(
        label: &str,
    ) -> (TerminalDaemon, SessionId, SessionId) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let path = std::env::temp_dir().join(format!(
            "terminal-platform-daemon-client-{label}-{}-{nanos}.sqlite3",
            std::process::id()
        ));
        let store =
            SqliteSessionStore::open(&path).expect("isolated sqlite session store should open");
        let valid_session_id = SessionId::new();
        let corrupt_session_id = SessionId::new();
        let tab_id = TabId::new();
        let pane_id = PaneId::new();
        let manifest = SavedSessionManifest::current();
        let route = local_native_route(valid_session_id);
        let launch = Some(cat_launch_spec());
        store
            .save_native_session(&terminal_persistence::SavedNativeSession {
                session_id: valid_session_id,
                route: route.clone(),
                title: Some("healthy-shell".to_string()),
                launch: launch.clone(),
                manifest: manifest.clone(),
                topology: TopologySnapshot {
                    session_id: valid_session_id,
                    backend_kind: BackendKind::Native,
                    tabs: vec![TabSnapshot {
                        tab_id,
                        title: Some("healthy-shell".to_string()),
                        root: PaneTreeNode::Leaf { pane_id },
                        focused_pane: Some(pane_id),
                    }],
                    focused_tab: Some(tab_id),
                },
                screens: Vec::new(),
                saved_at_ms: SqliteSessionStore::save_timestamp_ms()
                    .expect("save timestamp should resolve"),
            })
            .expect("valid snapshot should save");

        let connection = Connection::open(&path).expect("raw sqlite should open");
        connection
            .execute(
                "
                INSERT INTO native_saved_sessions (
                    session_id,
                    route_json,
                    title,
                    launch_json,
                    manifest_json,
                    topology_json,
                    screens_json,
                    saved_at_ms
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                ",
                params![
                    corrupt_session_id.0.to_string(),
                    serde_json::to_string(&route).expect("route should serialize"),
                    "corrupt-shell",
                    serde_json::to_string(&launch).expect("launch should serialize"),
                    serde_json::to_string(&manifest).expect("manifest should serialize"),
                    "{not-json",
                    serde_json::to_string::<Vec<terminal_projection::ScreenSnapshot>>(&Vec::new())
                        .expect("screens should serialize"),
                    SqliteSessionStore::save_timestamp_ms().expect("save timestamp should resolve")
                        + 1,
                ],
            )
            .expect("corrupted row should insert");

        (TerminalDaemon::with_persistence(store), valid_session_id, corrupt_session_id)
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

    fn submitted_input(text: &str) -> String {
        if cfg!(windows) { format!("echo {text}\r\n") } else { format!("{text}\n") }
    }

    async fn wait_for_screen_line(
        client: &LocalSocketDaemonClient,
        session_id: terminal_domain::SessionId,
        pane_id: terminal_domain::PaneId,
        needle: &str,
    ) {
        let mut last_lines = Vec::new();
        for _ in 0..120 {
            let screen = client
                .screen_snapshot(session_id, pane_id)
                .await
                .expect("screen_snapshot should succeed");
            if screen.surface.lines.iter().any(|line| line.text.contains(needle)) {
                return;
            }
            last_lines =
                screen.surface.lines.iter().map(|line| line.text.clone()).take(12).collect();
            sleep(Duration::from_millis(50)).await;
        }

        panic!("screen never contained expected text: {needle}; last lines: {last_lines:?}");
    }

    async fn recv_subscription_event(
        subscription: &mut super::LocalSocketSubscription,
    ) -> Option<SubscriptionEvent> {
        timeout(Duration::from_secs(5), subscription.recv())
            .await
            .expect("subscription recv should not hang")
            .expect("subscription recv should succeed")
    }

    async fn must_recv_subscription_event(
        subscription: &mut super::LocalSocketSubscription,
    ) -> SubscriptionEvent {
        recv_subscription_event(subscription).await.expect("subscription should emit an event")
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn roundtrips_handshake_and_empty_list_sessions() {
        let address = unique_address("daemon-client");
        let server = spawn_local_socket_server(TerminalDaemon::default(), address.clone())
            .expect("server should bind");
        let client = LocalSocketDaemonClient::new(address);

        let handshake = client.handshake().await.expect("handshake should succeed");
        let assessment =
            client.handshake_assessment().await.expect("handshake_assessment should succeed");
        let sessions = client.list_sessions().await.expect("list_sessions should succeed");

        assert_eq!(handshake.protocol_version.major, 0);
        assert_eq!(handshake.protocol_version.minor, 2);
        assert_eq!(handshake.daemon_phase, DaemonPhase::Ready);
        assert!(handshake.capabilities.request_reply);
        assert!(handshake.capabilities.topology_subscriptions);
        assert!(handshake.capabilities.pane_subscriptions);
        assert!(handshake.capabilities.backend_discovery);
        assert!(handshake.capabilities.backend_capability_queries);
        assert!(handshake.capabilities.saved_sessions);
        assert!(handshake.capabilities.session_restore);
        assert!(handshake.capabilities.degraded_error_reasons);
        assert!(handshake.capabilities.session_health);
        assert_eq!(client.info().expected_protocol, handshake.protocol_version);
        assert!(assessment.can_use);
        assert_eq!(assessment.status, HandshakeAssessmentStatus::Ready);
        assert!(sessions.sessions.is_empty());

        server.shutdown().await.expect("server shutdown should succeed");
    }

    #[test]
    fn assesses_handshake_protocol_and_phase() {
        let info = super::DaemonClientInfo::default();
        let starting = info.assess_handshake(&Handshake {
            protocol_version: ProtocolVersion { major: 0, minor: 2 },
            binary_version: CURRENT_BINARY_VERSION.to_string(),
            daemon_phase: DaemonPhase::Starting,
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
        });
        let incompatible = info.assess_handshake(&Handshake {
            protocol_version: ProtocolVersion { major: 0, minor: 3 },
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
        });

        assert!(!starting.can_use);
        assert_eq!(starting.status, HandshakeAssessmentStatus::Starting);
        assert!(starting.protocol.can_connect);
        assert!(!incompatible.can_use);
        assert_eq!(incompatible.status, HandshakeAssessmentStatus::ProtocolMinorAhead);
        assert!(!incompatible.protocol.can_connect);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn fetches_session_health_snapshot_for_native_session() {
        let address = unique_address("daemon-client-session-health");
        let server = spawn_local_socket_server(TerminalDaemon::default(), address.clone())
            .expect("server should bind");
        let client = LocalSocketDaemonClient::new(address);

        let created = client
            .create_session(BackendKind::Native, CreateSessionSpec::default())
            .await
            .expect("native session should be created");

        let health = client
            .session_health_snapshot(created.session.session_id)
            .await
            .expect("session health should succeed");

        assert_eq!(health.session_id, created.session.session_id);
        assert_eq!(health.phase, terminal_projection::SessionHealthPhase::Ready);
        assert!(health.can_attach);
        assert!(!health.invalidated);
        assert_eq!(health.reason, None);

        server.shutdown().await.expect("server shutdown should succeed");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn fetches_backend_capabilities() {
        let address = unique_address("daemon-client-capabilities");
        let server = spawn_local_socket_server(TerminalDaemon::default(), address.clone())
            .expect("server should bind");
        let client = LocalSocketDaemonClient::new(address);

        let native = client
            .backend_capabilities(BackendKind::Native)
            .await
            .expect("native capabilities should succeed");
        let zellij = client
            .backend_capabilities(BackendKind::Zellij)
            .await
            .expect("zellij capabilities should succeed");

        assert_eq!(native.backend, BackendKind::Native);
        assert!(native.capabilities.tiled_panes);
        assert!(native.capabilities.split_resize);
        assert!(native.capabilities.tab_create);
        assert!(native.capabilities.tab_close);
        assert!(native.capabilities.tab_focus);
        assert!(native.capabilities.tab_rename);
        assert!(native.capabilities.pane_split);
        assert!(native.capabilities.pane_close);
        assert!(native.capabilities.pane_focus);
        assert!(native.capabilities.pane_input_write);
        assert!(native.capabilities.layout_dump);
        assert!(native.capabilities.layout_override);
        assert!(native.capabilities.explicit_session_save);
        assert!(native.capabilities.explicit_session_restore);
        assert!(native.capabilities.rendered_viewport_stream);
        assert_eq!(zellij.backend, BackendKind::Zellij);
        assert!(zellij.capabilities.read_only_client_mode);

        server.shutdown().await.expect("server shutdown should succeed");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn creates_native_session_and_lists_it_back() {
        let address = unique_address("daemon-client-create");
        let server = spawn_local_socket_server(TerminalDaemon::default(), address.clone())
            .expect("server should bind");
        let client = LocalSocketDaemonClient::new(address);

        let created = client
            .create_session(
                BackendKind::Native,
                CreateSessionSpec {
                    title: Some("shell".to_string()),
                    ..CreateSessionSpec::default()
                },
            )
            .await
            .expect("create_session should succeed");
        let sessions = client.list_sessions().await.expect("list_sessions should succeed");

        assert_eq!(created.session.route.backend, BackendKind::Native);
        assert_eq!(created.session.title.as_deref(), Some("shell"));
        assert_eq!(sessions.sessions.len(), 1);
        assert_eq!(sessions.sessions[0].session_id, created.session.session_id);

        server.shutdown().await.expect("server shutdown should succeed");
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread")]
    async fn lists_and_loads_saved_native_sessions() {
        let address = unique_address("daemon-client-saved");
        let server = spawn_local_socket_server(TerminalDaemon::default(), address.clone())
            .expect("server should bind");
        let client = LocalSocketDaemonClient::new(address);
        let created = client
            .create_session(
                BackendKind::Native,
                CreateSessionSpec {
                    title: Some("shell".to_string()),
                    launch: Some(cat_launch_spec()),
                },
            )
            .await
            .expect("create_session should succeed");
        let topology = client
            .topology_snapshot(created.session.session_id)
            .await
            .expect("topology_snapshot should succeed");
        let pane_id = topology.tabs[0].focused_pane.expect("focused pane should exist");
        wait_for_screen_line(&client, created.session.session_id, pane_id, "ready").await;
        client
            .dispatch(created.session.session_id, MuxCommand::SaveSession)
            .await
            .expect("save session should succeed");

        let saved = client.list_saved_sessions().await.expect("list_saved_sessions should succeed");
        let saved_summary = saved
            .sessions
            .iter()
            .find(|session| session.session_id == created.session.session_id)
            .expect("saved session should be listed");
        let loaded = client
            .saved_session(created.session.session_id)
            .await
            .expect("saved_session should succeed");

        assert_eq!(saved_summary.route.backend, BackendKind::Native);
        assert_eq!(saved_summary.title.as_deref(), Some("shell"));
        assert_eq!(saved_summary.tab_count, 1);
        assert_eq!(saved_summary.pane_count, 1);
        assert!(saved_summary.has_launch);
        assert_eq!(saved_summary.manifest.format_version, 1);
        assert_eq!(saved_summary.manifest.binary_version, CURRENT_BINARY_VERSION);
        assert_eq!(saved_summary.manifest.protocol_major, CURRENT_PROTOCOL_MAJOR);
        assert_eq!(saved_summary.manifest.protocol_minor, CURRENT_PROTOCOL_MINOR);
        assert!(saved_summary.compatibility.can_restore);
        assert_eq!(saved_summary.compatibility.status, SavedSessionCompatibilityStatus::Compatible);
        assert!(saved_summary.restore_semantics.restores_topology);
        assert!(saved_summary.restore_semantics.uses_saved_launch_spec);
        assert!(!saved_summary.restore_semantics.replays_saved_screen_buffers);
        assert!(!saved_summary.restore_semantics.preserves_process_state);
        assert_eq!(loaded.session.session_id, created.session.session_id);
        assert_eq!(loaded.session.title.as_deref(), Some("shell"));
        assert_eq!(loaded.session.topology.tabs.len(), 1);
        assert_eq!(loaded.session.screens.len(), 1);
        assert_eq!(loaded.session.manifest.binary_version, CURRENT_BINARY_VERSION);
        assert!(loaded.session.compatibility.can_restore);
        assert_eq!(
            loaded.session.compatibility.status,
            SavedSessionCompatibilityStatus::Compatible
        );
        assert!(loaded.session.restore_semantics.restores_focus_state);
        assert!(loaded.session.restore_semantics.restores_tab_titles);
        assert!(!loaded.session.restore_semantics.replays_saved_screen_buffers);
        assert!(!loaded.session.restore_semantics.preserves_process_state);

        server.shutdown().await.expect("server shutdown should succeed");
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread")]
    async fn deletes_saved_native_sessions() {
        let address = unique_address("daemon-client-saved-delete");
        let server = spawn_local_socket_server(TerminalDaemon::default(), address.clone())
            .expect("server should bind");
        let client = LocalSocketDaemonClient::new(address);
        let created = client
            .create_session(
                BackendKind::Native,
                CreateSessionSpec {
                    title: Some("shell".to_string()),
                    launch: Some(cat_launch_spec()),
                },
            )
            .await
            .expect("create_session should succeed");
        let topology = client
            .topology_snapshot(created.session.session_id)
            .await
            .expect("topology_snapshot should succeed");
        let pane_id = topology.tabs[0].focused_pane.expect("focused pane should exist");
        wait_for_screen_line(&client, created.session.session_id, pane_id, "ready").await;
        client
            .dispatch(created.session.session_id, MuxCommand::SaveSession)
            .await
            .expect("save session should succeed");

        let deleted = client
            .delete_saved_session(created.session.session_id)
            .await
            .expect("delete_saved_session should succeed");
        let listed =
            client.list_saved_sessions().await.expect("list_saved_sessions should succeed");
        let lookup_error = client
            .saved_session(created.session.session_id)
            .await
            .expect_err("saved session lookup should fail after delete");

        assert_eq!(deleted.session_id, created.session.session_id);
        assert!(
            !listed.sessions.iter().any(|session| session.session_id == created.session.session_id)
        );
        assert_eq!(lookup_error.code, "backend_not_found");

        server.shutdown().await.expect("server shutdown should succeed");
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread")]
    async fn restores_saved_native_session_topology() {
        let address = unique_address("daemon-client-restore");
        let server = spawn_local_socket_server(TerminalDaemon::default(), address.clone())
            .expect("server should bind");
        let client = LocalSocketDaemonClient::new(address);
        let created = client
            .create_session(
                BackendKind::Native,
                CreateSessionSpec {
                    title: Some("shell".to_string()),
                    launch: Some(cat_launch_spec()),
                },
            )
            .await
            .expect("create_session should succeed");
        let initial = client
            .topology_snapshot(created.session.session_id)
            .await
            .expect("topology_snapshot should succeed");
        let first_pane = initial.tabs[0].focused_pane.expect("focused pane should exist");

        wait_for_screen_line(&client, created.session.session_id, first_pane, "ready").await;
        client
            .dispatch(
                created.session.session_id,
                MuxCommand::NewTab(NewTabSpec { title: Some("logs".to_string()) }),
            )
            .await
            .expect("new tab should succeed");
        let with_tabs = client
            .topology_snapshot(created.session.session_id)
            .await
            .expect("topology_snapshot should succeed");
        let second_tab_id = with_tabs.tabs[1].tab_id;
        client
            .dispatch(created.session.session_id, MuxCommand::FocusTab { tab_id: second_tab_id })
            .await
            .expect("focus tab should succeed");
        client
            .dispatch(created.session.session_id, MuxCommand::SaveSession)
            .await
            .expect("save session should succeed");

        let restored = client
            .restore_saved_session(created.session.session_id)
            .await
            .expect("restore_saved_session should succeed");
        let restored_topology = client
            .topology_snapshot(restored.session.session_id)
            .await
            .expect("topology_snapshot should succeed");

        assert_eq!(restored.saved_session_id, created.session.session_id);
        assert_ne!(restored.session.session_id, created.session.session_id);
        assert_eq!(restored.session.route.backend, BackendKind::Native);
        assert_eq!(restored.session.title.as_deref(), Some("logs"));
        assert_eq!(restored.manifest.binary_version, CURRENT_BINARY_VERSION);
        assert!(restored.compatibility.can_restore);
        assert_eq!(restored.compatibility.status, SavedSessionCompatibilityStatus::Compatible);
        assert!(restored.restore_semantics.restores_topology);
        assert!(restored.restore_semantics.uses_saved_launch_spec);
        assert!(!restored.restore_semantics.replays_saved_screen_buffers);
        assert!(!restored.restore_semantics.preserves_process_state);
        assert_eq!(restored_topology.tabs.len(), 2);
        let focused_tab = restored_topology.focused_tab.expect("focused tab should exist");
        let focused_tab = restored_topology
            .tabs
            .iter()
            .find(|tab| tab.tab_id == focused_tab)
            .expect("focused tab should exist");
        assert_eq!(focused_tab.title.as_deref(), Some("logs"));

        server.shutdown().await.expect("server shutdown should succeed");
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread")]
    async fn reports_incompatible_saved_sessions_and_blocks_restore() {
        let (daemon, session_id) = isolated_daemon_with_saved_snapshot(
            "daemon-client-saved-incompatible",
            SavedSessionManifest {
                format_version: CURRENT_SAVED_SESSION_FORMAT_VERSION,
                binary_version: CURRENT_BINARY_VERSION.to_string(),
                protocol_major: CURRENT_PROTOCOL_MAJOR,
                protocol_minor: CURRENT_PROTOCOL_MINOR + 1,
            },
        );
        let address = unique_address("daemon-client-saved-incompatible");
        let server =
            spawn_local_socket_server(daemon, address.clone()).expect("server should bind");
        let client = LocalSocketDaemonClient::new(address);

        let listed =
            client.list_saved_sessions().await.expect("list_saved_sessions should succeed");
        let listed_session = listed
            .sessions
            .iter()
            .find(|session| session.session_id == session_id)
            .expect("saved session should be listed");
        let loaded = client.saved_session(session_id).await.expect("saved_session should succeed");
        let restore_error = client
            .restore_saved_session(session_id)
            .await
            .expect_err("restore_saved_session should reject incompatible manifest");

        assert!(!listed_session.compatibility.can_restore);
        assert_eq!(
            listed_session.compatibility.status,
            SavedSessionCompatibilityStatus::ProtocolMinorAhead
        );
        assert!(!loaded.session.compatibility.can_restore);
        assert_eq!(
            loaded.session.compatibility.status,
            SavedSessionCompatibilityStatus::ProtocolMinorAhead
        );
        assert_eq!(restore_error.code, "backend_unsupported");
        assert_eq!(
            restore_error.degraded_reason,
            Some(DegradedModeReason::SavedSessionIncompatible)
        );

        server.shutdown().await.expect("server shutdown should succeed");
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread")]
    async fn list_saved_sessions_skips_corrupted_rows() {
        let (daemon, valid_session_id, corrupt_session_id) =
            isolated_daemon_with_valid_and_corrupted_saved_rows("daemon-client-saved-corrupt");
        let address = unique_address("daemon-client-saved-corrupt");
        let server =
            spawn_local_socket_server(daemon, address.clone()).expect("server should bind");
        let client = LocalSocketDaemonClient::new(address);

        let listed =
            client.list_saved_sessions().await.expect("list_saved_sessions should succeed");
        let loaded = client
            .saved_session(valid_session_id)
            .await
            .expect("valid saved_session should succeed");
        let corrupt_error = client
            .saved_session(corrupt_session_id)
            .await
            .expect_err("corrupted saved_session lookup should fail");

        assert_eq!(listed.sessions.len(), 1);
        assert_eq!(listed.sessions[0].session_id, valid_session_id);
        assert_eq!(listed.sessions[0].title.as_deref(), Some("healthy-shell"));
        assert_eq!(loaded.session.session_id, valid_session_id);
        assert_eq!(corrupt_error.code, "backend_internal");

        server.shutdown().await.expect("server shutdown should succeed");
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread")]
    async fn prunes_saved_native_sessions_to_latest_count() {
        let address = unique_address("daemon-client-prune-saved");
        let server = spawn_local_socket_server(isolated_daemon(), address.clone())
            .expect("server should bind");
        let client = LocalSocketDaemonClient::new(address);
        let mut last_saved_session = None;

        for title in ["shell-a", "shell-b", "shell-c"] {
            let created = client
                .create_session(
                    BackendKind::Native,
                    CreateSessionSpec {
                        title: Some(title.to_string()),
                        launch: Some(cat_launch_spec()),
                    },
                )
                .await
                .expect("create_session should succeed");
            let topology = client
                .topology_snapshot(created.session.session_id)
                .await
                .expect("topology_snapshot should succeed");
            let pane_id = topology.tabs[0].focused_pane.expect("focused pane should exist");
            wait_for_screen_line(&client, created.session.session_id, pane_id, "ready").await;
            client
                .dispatch(created.session.session_id, MuxCommand::SaveSession)
                .await
                .expect("save session should succeed");
            last_saved_session = Some(created.session.session_id);
            thread::sleep(Duration::from_millis(5));
        }

        let pruned =
            client.prune_saved_sessions(1).await.expect("prune_saved_sessions should succeed");
        let listed =
            client.list_saved_sessions().await.expect("list_saved_sessions should succeed");

        assert_eq!(pruned.deleted_count, 2);
        assert_eq!(pruned.kept_count, 1);
        assert_eq!(listed.sessions.len(), 1);
        assert_eq!(
            listed.sessions[0].session_id,
            last_saved_session.expect("saved session id should exist")
        );

        server.shutdown().await.expect("server shutdown should succeed");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn fetches_topology_and_screen_for_native_session() {
        let address = unique_address("daemon-client-topology");
        let server = spawn_local_socket_server(TerminalDaemon::default(), address.clone())
            .expect("server should bind");
        let client = LocalSocketDaemonClient::new(address);
        let created = client
            .create_session(
                BackendKind::Native,
                CreateSessionSpec {
                    title: Some("shell".to_string()),
                    ..CreateSessionSpec::default()
                },
            )
            .await
            .expect("create_session should succeed");
        let topology = client
            .topology_snapshot(created.session.session_id)
            .await
            .expect("topology_snapshot should succeed");
        let pane_id = topology.tabs[0].focused_pane.expect("focused pane should exist");
        let screen = client
            .screen_snapshot(created.session.session_id, pane_id)
            .await
            .expect("screen_snapshot should succeed");

        assert_eq!(topology.session_id, created.session.session_id);
        assert_eq!(screen.pane_id, pane_id);
        assert!(!screen.surface.lines.is_empty());

        server.shutdown().await.expect("server shutdown should succeed");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn dispatches_tab_mutations_and_observes_topology_change() {
        let address = unique_address("daemon-client-dispatch");
        let server = spawn_local_socket_server(TerminalDaemon::default(), address.clone())
            .expect("server should bind");
        let client = LocalSocketDaemonClient::new(address);
        let created = client
            .create_session(
                BackendKind::Native,
                CreateSessionSpec {
                    title: Some("shell".to_string()),
                    ..CreateSessionSpec::default()
                },
            )
            .await
            .expect("create_session should succeed");

        let result = client
            .dispatch(
                created.session.session_id,
                MuxCommand::NewTab(NewTabSpec { title: Some("logs".to_string()) }),
            )
            .await
            .expect("dispatch should succeed");
        let topology = client
            .topology_snapshot(created.session.session_id)
            .await
            .expect("topology snapshot should succeed");

        assert!(result.changed);
        assert_eq!(topology.tabs.len(), 2);
        assert_eq!(topology.focused_tab, Some(topology.tabs[1].tab_id));

        server.shutdown().await.expect("server shutdown should succeed");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn maps_backend_errors_for_invalid_close_tab_sequence() {
        let address = unique_address("daemon-client-errors");
        let server = spawn_local_socket_server(TerminalDaemon::default(), address.clone())
            .expect("server should bind");
        let client = LocalSocketDaemonClient::new(address);
        let created = client
            .create_session(
                BackendKind::Native,
                CreateSessionSpec {
                    title: Some("shell".to_string()),
                    ..CreateSessionSpec::default()
                },
            )
            .await
            .expect("create_session should succeed");
        let topology = client
            .topology_snapshot(created.session.session_id)
            .await
            .expect("topology_snapshot should succeed");
        let only_tab = topology.tabs[0].tab_id;
        let error = client
            .dispatch(created.session.session_id, MuxCommand::CloseTab { tab_id: only_tab })
            .await
            .expect_err("close last tab should fail");

        assert_eq!(error.code, "backend_invalid_input");

        server.shutdown().await.expect("server shutdown should succeed");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn fetches_screen_delta_for_native_session() {
        let address = unique_address("daemon-client-delta");
        let server = spawn_local_socket_server(TerminalDaemon::default(), address.clone())
            .expect("server should bind");
        let client = LocalSocketDaemonClient::new(address);
        let created = client
            .create_session(
                BackendKind::Native,
                CreateSessionSpec {
                    title: Some("shell".to_string()),
                    ..CreateSessionSpec::default()
                },
            )
            .await
            .expect("create_session should succeed");
        let topology = client
            .topology_snapshot(created.session.session_id)
            .await
            .expect("topology_snapshot should succeed");
        let pane_id = topology.tabs[0].focused_pane.expect("focused pane should exist");
        let snapshot = client
            .screen_snapshot(created.session.session_id, pane_id)
            .await
            .expect("screen_snapshot should succeed");
        let delta = client
            .screen_delta(created.session.session_id, pane_id, snapshot.sequence)
            .await
            .expect("screen_delta should succeed");

        assert_eq!(delta.pane_id, pane_id);
        assert_eq!(delta.from_sequence, snapshot.sequence);
        assert_eq!(delta.to_sequence, snapshot.sequence);
        assert_eq!(delta.rows, snapshot.rows);
        assert_eq!(delta.cols, snapshot.cols);
        assert!(delta.patch.is_none());
        assert!(delta.full_replace.is_none());

        server.shutdown().await.expect("server shutdown should succeed");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn observes_title_only_screen_delta_after_tab_rename() {
        let address = unique_address("daemon-client-title-delta");
        let server = spawn_local_socket_server(TerminalDaemon::default(), address.clone())
            .expect("server should bind");
        let client = LocalSocketDaemonClient::new(address);
        let created = client
            .create_session(
                BackendKind::Native,
                CreateSessionSpec {
                    title: Some("shell".to_string()),
                    ..CreateSessionSpec::default()
                },
            )
            .await
            .expect("create_session should succeed");
        let topology = client
            .topology_snapshot(created.session.session_id)
            .await
            .expect("topology_snapshot should succeed");
        let pane_id = topology.tabs[0].focused_pane.expect("focused pane should exist");
        let tab_id = topology.tabs[0].tab_id;
        let before = client
            .screen_snapshot(created.session.session_id, pane_id)
            .await
            .expect("screen_snapshot should succeed");

        let result = client
            .dispatch(
                created.session.session_id,
                MuxCommand::RenameTab { tab_id, title: "renamed".to_string() },
            )
            .await
            .expect("rename tab should succeed");
        let delta = client
            .screen_delta(created.session.session_id, pane_id, before.sequence)
            .await
            .expect("screen_delta should succeed");
        let listed = client.list_sessions().await.expect("list_sessions should succeed");
        let patch = delta.patch.expect("delta patch should exist");

        assert!(result.changed);
        assert_eq!(listed.sessions[0].title.as_deref(), Some("renamed"));
        assert!(delta.to_sequence > before.sequence);
        assert!(patch.title_changed);
        assert_eq!(patch.title.as_deref(), Some("renamed"));
        assert!(patch.line_updates.is_empty());
        assert!(delta.full_replace.is_none());

        server.shutdown().await.expect("server shutdown should succeed");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn streams_topology_updates_over_subscription_lane() {
        let address = unique_address("daemon-client-sub-topology");
        let server = spawn_local_socket_server(TerminalDaemon::default(), address.clone())
            .expect("server should bind");
        let client = LocalSocketDaemonClient::new(address);
        let created = client
            .create_session(
                BackendKind::Native,
                CreateSessionSpec {
                    title: Some("shell".to_string()),
                    ..CreateSessionSpec::default()
                },
            )
            .await
            .expect("create_session should succeed");
        let mut subscription = client
            .open_subscription(created.session.session_id, SubscriptionSpec::SessionTopology)
            .await
            .expect("subscription should open");

        let initial = must_recv_subscription_event(&mut subscription).await;
        let initial = match initial {
            SubscriptionEvent::TopologySnapshot(snapshot) => snapshot,
            other => panic!("unexpected initial event: {other:?}"),
        };
        let result = client
            .dispatch(
                created.session.session_id,
                MuxCommand::NewTab(NewTabSpec { title: Some("logs".to_string()) }),
            )
            .await
            .expect("dispatch should succeed");
        let updated = must_recv_subscription_event(&mut subscription).await;
        let updated = match updated {
            SubscriptionEvent::TopologySnapshot(snapshot) => snapshot,
            other => panic!("unexpected topology event: {other:?}"),
        };

        assert_eq!(initial.tabs.len(), 1);
        assert!(result.changed);
        assert_eq!(updated.tabs.len(), 2);

        server.shutdown().await.expect("server shutdown should succeed");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn closes_topology_subscription_lane_explicitly() {
        let address = unique_address("daemon-client-sub-close");
        let server = spawn_local_socket_server(TerminalDaemon::default(), address.clone())
            .expect("server should bind");
        let client = LocalSocketDaemonClient::new(address);
        let created = client
            .create_session(
                BackendKind::Native,
                CreateSessionSpec {
                    title: Some("shell".to_string()),
                    ..CreateSessionSpec::default()
                },
            )
            .await
            .expect("create_session should succeed");
        let mut subscription = client
            .open_subscription(created.session.session_id, SubscriptionSpec::SessionTopology)
            .await
            .expect("subscription should open");

        let initial = must_recv_subscription_event(&mut subscription).await;
        match initial {
            SubscriptionEvent::TopologySnapshot(_) => {}
            other => panic!("unexpected initial event: {other:?}"),
        }
        subscription.close().await.expect("close should succeed");
        assert!(recv_subscription_event(&mut subscription).await.is_none());

        server.shutdown().await.expect("server shutdown should succeed");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn closes_topology_subscription_lane_with_buffered_events() {
        let address = unique_address("daemon-client-sub-close-backlog");
        let server = spawn_local_socket_server(TerminalDaemon::default(), address.clone())
            .expect("server should bind");
        let client = LocalSocketDaemonClient::new(address);
        let created = client
            .create_session(
                BackendKind::Native,
                CreateSessionSpec {
                    title: Some("shell".to_string()),
                    ..CreateSessionSpec::default()
                },
            )
            .await
            .expect("create_session should succeed");
        let topology = client
            .topology_snapshot(created.session.session_id)
            .await
            .expect("topology_snapshot should succeed");
        let tab_id = topology.focused_tab.expect("focused tab should exist");
        let mut subscription = client
            .open_subscription(created.session.session_id, SubscriptionSpec::SessionTopology)
            .await
            .expect("subscription should open");

        let initial = must_recv_subscription_event(&mut subscription).await;
        assert!(matches!(initial, SubscriptionEvent::TopologySnapshot(_)));

        for revision in 0..24 {
            client
                .dispatch(
                    created.session.session_id,
                    MuxCommand::RenameTab { tab_id, title: format!("close-backlog-{revision}") },
                )
                .await
                .expect("rename tab should succeed");
        }

        subscription.close().await.expect("close should succeed");
        assert!(recv_subscription_event(&mut subscription).await.is_none());

        server.shutdown().await.expect("server shutdown should succeed");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn closes_topology_subscription_lane_when_server_shuts_down() {
        let address = unique_address("daemon-client-sub-server-shutdown");
        let server = spawn_local_socket_server(TerminalDaemon::default(), address.clone())
            .expect("server should bind");
        let client = LocalSocketDaemonClient::new(address);
        let created = client
            .create_session(
                BackendKind::Native,
                CreateSessionSpec {
                    title: Some("shell".to_string()),
                    ..CreateSessionSpec::default()
                },
            )
            .await
            .expect("create_session should succeed");
        let mut subscription = client
            .open_subscription(created.session.session_id, SubscriptionSpec::SessionTopology)
            .await
            .expect("subscription should open");

        let initial = must_recv_subscription_event(&mut subscription).await;
        match initial {
            SubscriptionEvent::TopologySnapshot(_) => {}
            other => panic!("unexpected initial event: {other:?}"),
        }

        server.shutdown().await.expect("server shutdown should succeed");

        let closed = timeout(Duration::from_secs(3), subscription.recv())
            .await
            .expect("subscription should close after server shutdown")
            .expect("recv should succeed");
        assert!(closed.is_none());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn closes_topology_subscription_lane_cleanly_after_server_shutdown() {
        let address = unique_address("sub-close-down");
        let server = spawn_local_socket_server(TerminalDaemon::default(), address.clone())
            .expect("server should bind");
        let client = LocalSocketDaemonClient::new(address);
        let created = client
            .create_session(
                BackendKind::Native,
                CreateSessionSpec {
                    title: Some("shell".to_string()),
                    ..CreateSessionSpec::default()
                },
            )
            .await
            .expect("create_session should succeed");
        let mut subscription = client
            .open_subscription(created.session.session_id, SubscriptionSpec::SessionTopology)
            .await
            .expect("subscription should open");

        let initial = must_recv_subscription_event(&mut subscription).await;
        assert!(matches!(initial, SubscriptionEvent::TopologySnapshot(_)));

        server.shutdown().await.expect("server shutdown should succeed");
        subscription.close().await.expect("close should tolerate a shutdown transport disconnect");
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread")]
    async fn streams_live_pane_surface_updates_over_subscription_lane() {
        let address = unique_address("daemon-client-sub-pane");
        let server = spawn_local_socket_server(TerminalDaemon::default(), address.clone())
            .expect("server should bind");
        let client = LocalSocketDaemonClient::new(address);
        let created = client
            .create_session(
                BackendKind::Native,
                CreateSessionSpec {
                    title: Some("shell".to_string()),
                    launch: Some(cat_launch_spec()),
                },
            )
            .await
            .expect("create_session should succeed");
        let topology = client
            .topology_snapshot(created.session.session_id)
            .await
            .expect("topology_snapshot should succeed");
        let pane_id = topology.tabs[0].focused_pane.expect("focused pane should exist");
        wait_for_screen_line(&client, created.session.session_id, pane_id, "ready").await;
        let mut subscription = client
            .open_subscription(
                created.session.session_id,
                SubscriptionSpec::PaneSurface { pane_id },
            )
            .await
            .expect("subscription should open");

        let initial = must_recv_subscription_event(&mut subscription).await;
        let initial = match initial {
            SubscriptionEvent::ScreenDelta(delta) => delta,
            other => panic!("unexpected initial event: {other:?}"),
        };
        let result = client
            .dispatch(
                created.session.session_id,
                MuxCommand::SendInput(SendInputSpec {
                    pane_id,
                    data: submitted_input("hello from subscription"),
                }),
            )
            .await
            .expect("dispatch should succeed");
        let updated = must_recv_subscription_event(&mut subscription).await;
        let updated = match updated {
            SubscriptionEvent::ScreenDelta(delta) => delta,
            other => panic!("unexpected screen event: {other:?}"),
        };
        let patch = updated.patch.expect("delta patch should exist");

        assert!(!result.changed);
        assert!(initial.full_replace.is_some());
        assert!(updated.to_sequence > updated.from_sequence);
        assert!(
            patch
                .line_updates
                .iter()
                .any(|line| line.line.text.contains("hello from subscription"))
        );
        assert!(updated.full_replace.is_none());

        server.shutdown().await.expect("server shutdown should succeed");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn restarts_server_on_same_address_across_multiple_cycles() {
        let address = unique_address("daemon-client-restart-cycles");
        let client = LocalSocketDaemonClient::new(address.clone());

        for cycle in 0..3 {
            let server =
                spawn_default_daemon_with_retry(address.clone()).expect("server should bind");

            let handshake = client.handshake().await.expect("handshake should succeed");
            assert_eq!(handshake.daemon_phase, DaemonPhase::Ready, "cycle {cycle} should be ready");

            server.shutdown().await.expect("server shutdown should succeed");
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn repeatedly_opens_and_closes_topology_subscriptions() {
        let address = unique_address("daemon-client-subscribe-cycles");
        let server = spawn_local_socket_server(TerminalDaemon::default(), address.clone())
            .expect("server should bind");
        let client = LocalSocketDaemonClient::new(address);
        let created = client
            .create_session(
                BackendKind::Native,
                CreateSessionSpec {
                    title: Some("subscribe-cycles".to_string()),
                    ..CreateSessionSpec::default()
                },
            )
            .await
            .expect("create_session should succeed");

        for cycle in 0..24 {
            let mut subscription = client
                .open_subscription(created.session.session_id, SubscriptionSpec::SessionTopology)
                .await
                .expect("subscription should open");
            let initial = recv_subscription_event(&mut subscription).await;

            assert!(
                matches!(initial, Some(SubscriptionEvent::TopologySnapshot(_))),
                "cycle {cycle} should receive initial topology snapshot"
            );

            subscription.close().await.expect("subscription should close cleanly");
        }

        server.shutdown().await.expect("server shutdown should succeed");
    }
}
