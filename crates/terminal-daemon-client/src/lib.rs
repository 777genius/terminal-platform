use futures_util::{SinkExt as _, StreamExt as _};
use interprocess::local_socket::{tokio::Stream, traits::tokio::Stream as _};
use terminal_backend_api::{CreateSessionSpec, MuxCommand, MuxCommandResult, SubscriptionSpec};
use terminal_domain::{BackendKind, OperationId, SessionRoute};
use tokio_util::codec::{Framed, LengthDelimitedCodec};

use terminal_projection::{ScreenDelta, ScreenSnapshot, TopologySnapshot};
use terminal_protocol::{
    BackendCapabilitiesResponse, CreateSessionRequest, CreateSessionResponse,
    DeleteSavedSessionRequest, DeleteSavedSessionResponse, DiscoverSessionsRequest,
    DiscoverSessionsResponse, DispatchMuxCommandRequest, GetBackendCapabilitiesRequest,
    GetSavedSessionRequest, GetScreenDeltaRequest, GetScreenSnapshotRequest,
    GetTopologySnapshotRequest, Handshake, ImportSessionRequest, ImportSessionResponse,
    ListSavedSessionsResponse, ListSessionsResponse, LocalSocketAddress, OpenSubscriptionRequest,
    OpenSubscriptionResponse, ProtocolError, ProtocolVersion, PruneSavedSessionsRequest,
    PruneSavedSessionsResponse, RequestEnvelope, RequestPayload, ResponsePayload,
    RestoreSavedSessionRequest, RestoreSavedSessionResponse, SavedSessionResponse,
    SubscriptionEnvelope, SubscriptionEvent, SubscriptionRequest, SubscriptionRequestEnvelope,
    TransportResponse, decode_json_frame, encode_json_frame,
};

type LocalFramedStream = Framed<Stream, LengthDelimitedCodec>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonClientInfo {
    pub expected_protocol: ProtocolVersion,
}

impl Default for DaemonClientInfo {
    fn default() -> Self {
        Self { expected_protocol: ProtocolVersion { major: 0, minor: 1 } }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalSocketDaemonClient {
    address: LocalSocketAddress,
    info: DaemonClientInfo,
}

pub struct LocalSocketSubscription {
    subscription_id: terminal_domain::SubscriptionId,
    framed: LocalFramedStream,
}

impl LocalSocketSubscription {
    #[must_use]
    pub fn subscription_id(&self) -> terminal_domain::SubscriptionId {
        self.subscription_id
    }

    pub async fn recv(&mut self) -> Result<Option<SubscriptionEvent>, ProtocolError> {
        let Some(frame) = self.framed.next().await else {
            return Ok(None);
        };
        let frame = frame.map_err(|error| ProtocolError::io("receive_failed", &error))?;
        let envelope = decode_json_frame::<SubscriptionEnvelope>(&frame)?;
        if envelope.subscription_id != self.subscription_id {
            return Err(ProtocolError::new(
                "subscription_mismatch",
                format!(
                    "expected subscription {:?}, got {:?}",
                    self.subscription_id, envelope.subscription_id
                ),
            ));
        }

        Ok(Some(envelope.event))
    }

    pub async fn close(&mut self) -> Result<(), ProtocolError> {
        let request = SubscriptionRequestEnvelope {
            subscription_id: self.subscription_id,
            request: SubscriptionRequest::Close,
        };
        let encoded_request = encode_json_frame(&request)?;
        self.framed
            .send(encoded_request)
            .await
            .map_err(|error| ProtocolError::io("send_failed", &error))?;
        let closed = self
            .framed
            .next()
            .await
            .transpose()
            .map_err(|error| ProtocolError::io("receive_failed", &error))?;
        if closed.is_some() {
            return Err(ProtocolError::new(
                "unexpected_payload",
                "expected subscription stream to close after close request",
            ));
        }

        Ok(())
    }
}

impl LocalSocketDaemonClient {
    #[must_use]
    pub fn new(address: LocalSocketAddress) -> Self {
        Self { address, info: DaemonClientInfo::default() }
    }

    #[must_use]
    pub fn address(&self) -> &LocalSocketAddress {
        &self.address
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
        let operation_id = OperationId::new();
        let request = RequestEnvelope {
            operation_id,
            payload: RequestPayload::OpenSubscription(OpenSubscriptionRequest { session_id, spec }),
        };
        let encoded_request = encode_json_frame(&request)?;
        let stream = Stream::connect(
            self.address
                .to_name()
                .map_err(|error| ProtocolError::io("invalid_socket_name", &error))?,
        )
        .await
        .map_err(|error| ProtocolError::io("connect_failed", &error))?;
        let mut framed = Framed::new(stream, LengthDelimitedCodec::new());

        framed
            .send(encoded_request)
            .await
            .map_err(|error| ProtocolError::io("send_failed", &error))?;
        let Some(frame) = framed.next().await else {
            return Err(ProtocolError::new("unexpected_eof", "daemon closed stream"));
        };
        let frame = frame.map_err(|error| ProtocolError::io("receive_failed", &error))?;
        let response = decode_json_frame::<TransportResponse>(&frame)?.into_result()?;

        if response.operation_id != operation_id {
            return Err(ProtocolError::new(
                "operation_mismatch",
                format!(
                    "expected response for operation {:?}, got {:?}",
                    operation_id, response.operation_id
                ),
            ));
        }

        let subscription_id = match response.payload {
            ResponsePayload::SubscriptionOpened(OpenSubscriptionResponse { subscription_id }) => {
                subscription_id
            }
            other => return Err(ProtocolError::unexpected_payload("subscription_opened", &other)),
        };

        Ok(LocalSocketSubscription { subscription_id, framed })
    }

    async fn send_request(
        &self,
        payload: RequestPayload,
    ) -> Result<terminal_protocol::ResponseEnvelope, ProtocolError> {
        let operation_id = OperationId::new();
        let request = RequestEnvelope { operation_id, payload };
        let encoded_request = encode_json_frame(&request)?;
        let stream = Stream::connect(
            self.address
                .to_name()
                .map_err(|error| ProtocolError::io("invalid_socket_name", &error))?,
        )
        .await
        .map_err(|error| ProtocolError::io("connect_failed", &error))?;
        let mut framed = Framed::new(stream, LengthDelimitedCodec::new());

        framed
            .send(encoded_request)
            .await
            .map_err(|error| ProtocolError::io("send_failed", &error))?;

        let frame = framed
            .next()
            .await
            .ok_or_else(|| ProtocolError::new("unexpected_eof", "daemon closed stream"))?
            .map_err(|error| ProtocolError::io("receive_failed", &error))?;
        let response = decode_json_frame::<TransportResponse>(&frame)?.into_result()?;

        if response.operation_id != operation_id {
            return Err(ProtocolError::new(
                "operation_mismatch",
                format!(
                    "expected response for operation {:?}, got {:?}",
                    operation_id, response.operation_id
                ),
            ));
        }

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        thread,
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

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
    use terminal_protocol::SubscriptionEvent;

    use super::LocalSocketDaemonClient;

    fn unique_address(label: &str) -> terminal_protocol::LocalSocketAddress {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let slug = format!("terminal-platform-{label}-{}-{nanos}.sock", std::process::id());

        terminal_protocol::LocalSocketAddress::from_runtime_slug(slug)
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

        TerminalDaemon::new(terminal_daemon::TerminalDaemonState::with_default_persistence(store))
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

        (
            TerminalDaemon::new(terminal_daemon::TerminalDaemonState::with_default_persistence(
                store,
            )),
            session_id,
        )
    }

    #[cfg(unix)]
    fn cat_launch_spec() -> ShellLaunchSpec {
        ShellLaunchSpec::new("/bin/sh").with_args(["-lc", "printf 'ready\\n'; exec cat"])
    }

    #[cfg(unix)]
    async fn wait_for_screen_line(
        client: &LocalSocketDaemonClient,
        session_id: terminal_domain::SessionId,
        pane_id: terminal_domain::PaneId,
        needle: &str,
    ) {
        for _ in 0..40 {
            let screen = client
                .screen_snapshot(session_id, pane_id)
                .await
                .expect("screen_snapshot should succeed");
            if screen.surface.lines.iter().any(|line| line.text.contains(needle)) {
                return;
            }
            thread::sleep(Duration::from_millis(50));
        }

        panic!("screen never contained expected text: {needle}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn roundtrips_handshake_and_empty_list_sessions() {
        let address = unique_address("daemon-client");
        let server = spawn_local_socket_server(TerminalDaemon::default(), address.clone())
            .expect("server should bind");
        let client = LocalSocketDaemonClient::new(address);

        let handshake = client.handshake().await.expect("handshake should succeed");
        let sessions = client.list_sessions().await.expect("list_sessions should succeed");

        assert_eq!(handshake.protocol_version.major, 0);
        assert_eq!(handshake.protocol_version.minor, 1);
        assert_eq!(client.info().expected_protocol, handshake.protocol_version);
        assert!(sessions.sessions.is_empty());

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

        let initial = subscription.recv().await.expect("recv should succeed").expect("event");
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
        let updated = subscription.recv().await.expect("recv should succeed").expect("event");
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

        let initial = subscription.recv().await.expect("recv should succeed").expect("event");
        match initial {
            SubscriptionEvent::TopologySnapshot(_) => {}
            other => panic!("unexpected initial event: {other:?}"),
        }
        subscription.close().await.expect("close should succeed");
        assert!(subscription.recv().await.expect("recv should succeed").is_none());

        server.shutdown().await.expect("server shutdown should succeed");
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

        let initial = subscription.recv().await.expect("recv should succeed").expect("event");
        let initial = match initial {
            SubscriptionEvent::ScreenDelta(delta) => delta,
            other => panic!("unexpected initial event: {other:?}"),
        };
        let result = client
            .dispatch(
                created.session.session_id,
                MuxCommand::SendInput(SendInputSpec {
                    pane_id,
                    data: "hello from subscription\r".to_string(),
                }),
            )
            .await
            .expect("dispatch should succeed");
        let updated = subscription.recv().await.expect("recv should succeed").expect("event");
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
}
