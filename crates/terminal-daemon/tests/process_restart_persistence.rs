#![cfg(feature = "native-backend")]

use std::{
    path::Path,
    process::{Child, Command, Stdio},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use terminal_backend_api::{CreateSessionSpec, MuxCommand, SendInputSpec, ShellLaunchSpec};
use terminal_domain::{BackendKind, PaneId, SessionId};
use terminal_projection::{ScreenSnapshot, TopologySnapshot};
use terminal_protocol::{
    CreateSessionRequest, CreateSessionResponse, DispatchMuxCommandRequest, GetPaneHistoryRequest,
    GetSavedSessionRequest, GetScreenSnapshotRequest, GetTopologySnapshotRequest,
    LocalSocketAddress, PaneHistoryResponse, RequestPayload, ResponsePayload,
    RestoreSavedSessionRequest, RestoreSavedSessionResponse, SavedSessionResponse,
};
use terminal_transport::LocalSocketTransportClient;

#[tokio::test(flavor = "multi_thread")]
async fn daemon_process_kill_preserves_v2_output_history_after_restart() {
    let suffix = unique_suffix("daemon-process-kill-history");
    let runtime_slug = format!("terminal-platform-{suffix}");
    let store_path = std::env::temp_dir().join(format!("{suffix}.sqlite3"));
    let marker = format!("KILLHIST{}", short_suffix());
    let mut daemon = DaemonProcess::spawn(&runtime_slug, &store_path);
    let client = LocalSocketTransportClient::new(LocalSocketAddress::from_runtime_slug(
        runtime_slug.clone(),
    ));

    wait_for_daemon_ready(&client, &mut daemon).await;
    let created = create_native_shell_session(&client).await;
    let topology = topology_snapshot(&client, created.session.session_id).await;
    let pane_id = topology.tabs[0].focused_pane.expect("created session should have focused pane");
    wait_for_screen_line(&client, created.session.session_id, pane_id, "ready").await;
    send_input(&client, created.session.session_id, pane_id, &format!("echo {marker}")).await;
    wait_for_screen_line(&client, created.session.session_id, pane_id, &marker).await;
    wait_for_pane_history_line(&client, created.session.session_id, pane_id, &marker).await;
    send_input(&client, created.session.session_id, pane_id, "exit").await;
    tokio::time::sleep(Duration::from_millis(250)).await;

    daemon.kill_and_wait();

    let mut restarted = DaemonProcess::spawn(&runtime_slug, &store_path);
    wait_for_daemon_ready(&client, &mut restarted).await;
    let history =
        wait_for_pane_history_line(&client, created.session.session_id, pane_id, &marker).await;

    assert_eq!(history.session_id, created.session.session_id);
    assert_eq!(history.pane_id, pane_id);
    assert!(history.total_payload_bytes > 0);

    restarted.kill_and_wait();
}

#[tokio::test(flavor = "multi_thread")]
async fn daemon_process_restart_restores_saved_session_with_v2_history_evidence() {
    let suffix = unique_suffix("daemon-process-restore-history");
    let runtime_slug = format!("terminal-platform-{suffix}");
    let store_path = std::env::temp_dir().join(format!("{suffix}.sqlite3"));
    let marker = format!("KILLRESTORE{}", short_suffix());
    let mut daemon = DaemonProcess::spawn(&runtime_slug, &store_path);
    let client = LocalSocketTransportClient::new(LocalSocketAddress::from_runtime_slug(
        runtime_slug.clone(),
    ));

    wait_for_daemon_ready(&client, &mut daemon).await;
    let created = create_native_shell_session(&client).await;
    let topology = topology_snapshot(&client, created.session.session_id).await;
    let pane_id = topology.tabs[0].focused_pane.expect("created session should have focused pane");

    wait_for_screen_line(&client, created.session.session_id, pane_id, "ready").await;
    send_input(&client, created.session.session_id, pane_id, &format!("echo {marker}")).await;
    wait_for_screen_line(&client, created.session.session_id, pane_id, &marker).await;
    wait_for_pane_history_line(&client, created.session.session_id, pane_id, &marker).await;
    save_session(&client, created.session.session_id).await;
    let saved_before_restart = saved_session(&client, created.session.session_id).await;
    let saved_v2_before_restart = saved_before_restart
        .session
        .restore_semantics_v2
        .as_ref()
        .expect("saved session should expose v2 restore semantics before restart");

    assert_eq!(saved_v2_before_restart.source_session_id, created.session.session_id);
    assert_eq!(saved_v2_before_restart.restored_session_id, None);
    assert!(!saved_v2_before_restart.evidence_refs.is_empty());

    send_input(&client, created.session.session_id, pane_id, "exit").await;
    tokio::time::sleep(Duration::from_millis(250)).await;
    daemon.kill_and_wait();

    let mut restarted = DaemonProcess::spawn(&runtime_slug, &store_path);
    wait_for_daemon_ready(&client, &mut restarted).await;
    let saved_after_restart = saved_session(&client, created.session.session_id).await;
    let saved_v2_after_restart = saved_after_restart
        .session
        .restore_semantics_v2
        .as_ref()
        .expect("saved session should expose v2 restore semantics after restart");
    let restored = restore_saved_session(&client, created.session.session_id).await;
    let restored_v2 = restored
        .restore_semantics_v2
        .as_ref()
        .expect("restore response should expose v2 restore semantics after restart");
    let restored_topology = topology_snapshot(&client, restored.session.session_id).await;
    let restored_pane_id =
        restored_topology.tabs[0].focused_pane.expect("restored session should have focused pane");
    let history =
        wait_for_pane_history_line(&client, created.session.session_id, pane_id, &marker).await;

    assert_eq!(saved_after_restart.session.session_id, created.session.session_id);
    assert_eq!(saved_v2_after_restart.source_session_id, created.session.session_id);
    assert_eq!(saved_v2_after_restart.restored_session_id, None);
    assert!(!saved_v2_after_restart.evidence_refs.is_empty());
    assert_eq!(restored.saved_session_id, created.session.session_id);
    assert_ne!(restored.session.session_id, created.session.session_id);
    assert_eq!(restored_v2.source_session_id, created.session.session_id);
    assert_eq!(restored_v2.restored_session_id, Some(restored.session.session_id));
    assert!(!restored_v2.evidence_refs.is_empty());
    assert_eq!(history.session_id, created.session.session_id);
    assert_eq!(history.pane_id, pane_id);

    wait_for_screen_line(&client, restored.session.session_id, restored_pane_id, "ready").await;

    restarted.kill_and_wait();
}

struct DaemonProcess {
    child: Child,
}

impl DaemonProcess {
    fn spawn(runtime_slug: &str, store_path: &Path) -> Self {
        let mut command = Command::new(env!("CARGO_BIN_EXE_terminal-daemon"));
        command
            .arg("--runtime-slug")
            .arg(runtime_slug)
            .arg("--session-store")
            .arg(store_path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        hide_console_window(&mut command);
        let child = command.spawn().expect("terminal-daemon process should spawn");

        Self { child }
    }

    fn kill_and_wait(&mut self) {
        if self.child.try_wait().expect("daemon child status should be readable").is_none() {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
    }

    fn assert_still_running(&mut self) {
        if let Some(status) = self.child.try_wait().expect("daemon child status should be readable")
        {
            panic!("terminal-daemon exited early with status {status}");
        }
    }
}

impl Drop for DaemonProcess {
    fn drop(&mut self) {
        self.kill_and_wait();
    }
}

#[cfg(windows)]
fn hide_console_window(command: &mut Command) {
    use std::os::windows::process::CommandExt as _;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn hide_console_window(_command: &mut Command) {}

async fn wait_for_daemon_ready(client: &LocalSocketTransportClient, daemon: &mut DaemonProcess) {
    let started = std::time::Instant::now();
    while started.elapsed() < Duration::from_secs(10) {
        daemon.assert_still_running();
        if matches!(
            client.send_request(RequestPayload::Handshake).await,
            Ok(response) if matches!(response.payload, ResponsePayload::Handshake(_))
        ) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    panic!("terminal-daemon process did not become ready");
}

async fn create_native_shell_session(client: &LocalSocketTransportClient) -> CreateSessionResponse {
    let response = client
        .send_request(RequestPayload::CreateSession(CreateSessionRequest {
            backend: BackendKind::Native,
            spec: CreateSessionSpec {
                title: Some("process-kill-history".to_string()),
                launch: Some(shell_launch()),
            },
        }))
        .await
        .expect("create session request should succeed");

    match response.payload {
        ResponsePayload::CreateSession(created) => created,
        other => panic!("unexpected create session response: {other:?}"),
    }
}

fn shell_launch() -> ShellLaunchSpec {
    #[cfg(windows)]
    {
        let program = std::env::var("COMSPEC")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "cmd.exe".to_string());
        ShellLaunchSpec::new(program).with_args([
            "/D".to_string(),
            "/Q".to_string(),
            "/K".to_string(),
            "echo ready".to_string(),
        ])
    }

    #[cfg(unix)]
    {
        ShellLaunchSpec::new("/bin/sh")
            .with_args(["-lc".to_string(), "printf 'ready\\n'; exec /bin/sh".to_string()])
    }
}

async fn send_input(
    client: &LocalSocketTransportClient,
    session_id: SessionId,
    pane_id: PaneId,
    command: &str,
) {
    let response = client
        .send_request(RequestPayload::DispatchMuxCommand(DispatchMuxCommandRequest {
            session_id,
            command: MuxCommand::SendInput(SendInputSpec {
                pane_id,
                data: submitted_input(command),
                client_event_id: None,
            }),
        }))
        .await
        .expect("send input request should succeed");

    match response.payload {
        ResponsePayload::DispatchMuxCommand(_) => {}
        other => panic!("unexpected dispatch response: {other:?}"),
    }
}

async fn save_session(client: &LocalSocketTransportClient, session_id: SessionId) {
    let response = client
        .send_request(RequestPayload::DispatchMuxCommand(DispatchMuxCommandRequest {
            session_id,
            command: MuxCommand::SaveSession,
        }))
        .await
        .expect("save session request should succeed");

    match response.payload {
        ResponsePayload::DispatchMuxCommand(_) => {}
        other => panic!("unexpected save session response: {other:?}"),
    }
}

async fn saved_session(
    client: &LocalSocketTransportClient,
    session_id: SessionId,
) -> SavedSessionResponse {
    let response = client
        .send_request(RequestPayload::GetSavedSession(GetSavedSessionRequest { session_id }))
        .await
        .expect("saved session request should succeed");

    match response.payload {
        ResponsePayload::SavedSession(saved) => saved,
        other => panic!("unexpected saved session response: {other:?}"),
    }
}

async fn restore_saved_session(
    client: &LocalSocketTransportClient,
    session_id: SessionId,
) -> RestoreSavedSessionResponse {
    let response = client
        .send_request(RequestPayload::RestoreSavedSession(RestoreSavedSessionRequest {
            session_id,
        }))
        .await
        .expect("restore saved session request should succeed");

    match response.payload {
        ResponsePayload::RestoreSavedSession(restored) => restored,
        other => panic!("unexpected restore saved session response: {other:?}"),
    }
}

fn submitted_input(command: &str) -> String {
    #[cfg(windows)]
    {
        format!("{command}\r")
    }

    #[cfg(unix)]
    {
        format!("{command}\n")
    }
}

async fn topology_snapshot(
    client: &LocalSocketTransportClient,
    session_id: SessionId,
) -> TopologySnapshot {
    let response = client
        .send_request(RequestPayload::GetTopologySnapshot(GetTopologySnapshotRequest {
            session_id,
        }))
        .await
        .expect("topology snapshot request should succeed");

    match response.payload {
        ResponsePayload::TopologySnapshot(topology) => topology,
        other => panic!("unexpected topology snapshot response: {other:?}"),
    }
}

async fn wait_for_screen_line(
    client: &LocalSocketTransportClient,
    session_id: SessionId,
    pane_id: PaneId,
    marker: &str,
) -> ScreenSnapshot {
    let started = std::time::Instant::now();
    while started.elapsed() < Duration::from_secs(10) {
        if let Ok(response) = client
            .send_request(RequestPayload::GetScreenSnapshot(GetScreenSnapshotRequest {
                session_id,
                pane_id,
            }))
            .await
        {
            if let ResponsePayload::ScreenSnapshot(snapshot) = response.payload {
                if snapshot.surface.lines.iter().any(|line| line.text.contains(marker)) {
                    return snapshot;
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    panic!("screen marker {marker} did not appear");
}

async fn wait_for_pane_history_line(
    client: &LocalSocketTransportClient,
    session_id: SessionId,
    pane_id: PaneId,
    marker: &str,
) -> PaneHistoryResponse {
    let started = std::time::Instant::now();
    while started.elapsed() < Duration::from_secs(10) {
        if let Ok(response) = client
            .send_request(RequestPayload::GetPaneHistory(GetPaneHistoryRequest {
                session_id,
                pane_id,
                from_event_seq: Some(1),
                max_segments: Some(64),
                max_bytes: Some(512 * 1024),
            }))
            .await
        {
            if let ResponsePayload::PaneHistory(history) = response.payload {
                if history
                    .segments
                    .iter()
                    .any(|segment| String::from_utf8_lossy(&segment.payload).contains(marker))
                {
                    return history;
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    panic!("pane history marker {marker} did not appear after daemon process restart");
}

fn unique_suffix(label: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("{label}-{}-{nanos}", std::process::id())
}

fn short_suffix() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("{:08x}", (nanos & 0xffff_ffff) as u32)
}
