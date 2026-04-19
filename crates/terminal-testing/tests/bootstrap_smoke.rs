use std::{thread, time::Duration};

#[cfg(unix)]
use std::{
    process::Command,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(unix)]
use terminal_application::BackendCatalog;
use terminal_backend_api::{
    CreateSessionSpec, MuxBackendPort, MuxCommand, NewTabSpec, SendInputSpec, ShellLaunchSpec,
    SubscriptionSpec,
};
#[cfg(unix)]
use terminal_backend_native::NativeBackend;
#[cfg(unix)]
use terminal_backend_tmux::TmuxBackend;
#[cfg(unix)]
use terminal_backend_zellij::ZellijBackend;
#[cfg(unix)]
use terminal_daemon::TerminalDaemonState;
use terminal_domain::BackendKind;
#[cfg(unix)]
use terminal_projection::ProjectionSource;
use terminal_protocol::SubscriptionEvent;
use terminal_testing::{daemon_fixture, daemon_fixture_with_state, daemon_state};

#[test]
fn bootstrap_smoke_exposes_empty_daemon_state() {
    let daemon = daemon_state();
    let handshake = daemon.handshake();

    assert_eq!(handshake.protocol_version.major, 0);
    assert_eq!(handshake.protocol_version.minor, 1);
    assert_eq!(handshake.available_backends.len(), 3);
    assert_eq!(daemon.session_count(), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_smoke_roundtrips_request_reply_flow() {
    let fixture = daemon_fixture("bootstrap-smoke").expect("fixture should start");
    let created = fixture
        .client
        .create_session(
            BackendKind::Native,
            CreateSessionSpec { title: Some("shell".to_string()), ..CreateSessionSpec::default() },
        )
        .await
        .expect("create_session should succeed");
    let handshake = fixture.client.handshake().await.expect("handshake should succeed");
    let sessions = fixture.client.list_sessions().await.expect("list_sessions should succeed");
    let topology = fixture
        .client
        .topology_snapshot(created.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let pane_id = topology.tabs[0].focused_pane.expect("focused pane should exist");
    let screen = fixture
        .client
        .screen_snapshot(created.session.session_id, pane_id)
        .await
        .expect("screen_snapshot should succeed");
    let delta = fixture
        .client
        .screen_delta(created.session.session_id, pane_id, screen.sequence)
        .await
        .expect("screen_delta should succeed");
    let dispatch = fixture
        .client
        .dispatch(
            created.session.session_id,
            MuxCommand::NewTab(NewTabSpec { title: Some("logs".to_string()) }),
        )
        .await
        .expect("dispatch should succeed");
    let topology_after_dispatch = fixture
        .client
        .topology_snapshot(created.session.session_id)
        .await
        .expect("topology_snapshot should succeed");

    assert_eq!(handshake.protocol_version.major, 0);
    assert_eq!(handshake.protocol_version.minor, 1);
    assert_eq!(created.session.route.backend, BackendKind::Native);
    assert_eq!(created.session.title.as_deref(), Some("shell"));
    assert_eq!(sessions.sessions.len(), 1);
    assert_eq!(sessions.sessions[0].session_id, created.session.session_id);
    assert_eq!(topology.session_id, created.session.session_id);
    assert_eq!(screen.pane_id, pane_id);
    assert!(!screen.surface.lines.is_empty());
    assert_eq!(delta.rows, screen.rows);
    assert_eq!(delta.cols, screen.cols);
    assert!(delta.patch.is_none());
    assert_eq!(delta.from_sequence, screen.sequence);
    assert_eq!(delta.to_sequence, screen.sequence);
    assert!(delta.full_replace.is_none());
    assert!(dispatch.changed);
    assert_eq!(topology_after_dispatch.tabs.len(), 2);

    fixture.shutdown().await.expect("fixture should stop cleanly");
}

#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_smoke_streams_topology_updates() {
    let fixture = daemon_fixture("bootstrap-subscription-smoke").expect("fixture should start");
    let created = fixture
        .client
        .create_session(
            BackendKind::Native,
            CreateSessionSpec { title: Some("shell".to_string()), ..CreateSessionSpec::default() },
        )
        .await
        .expect("create_session should succeed");
    let mut subscription = fixture
        .client
        .open_subscription(created.session.session_id, SubscriptionSpec::SessionTopology)
        .await
        .expect("subscription should open");

    let initial = subscription.recv().await.expect("recv should succeed").expect("event");
    let initial = match initial {
        SubscriptionEvent::TopologySnapshot(snapshot) => snapshot,
        other => panic!("unexpected initial event: {other:?}"),
    };
    let dispatch = fixture
        .client
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
    assert!(dispatch.changed);
    assert_eq!(updated.tabs.len(), 2);

    fixture.shutdown().await.expect("fixture should stop cleanly");
}

#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_smoke_closes_subscription_lane_explicitly() {
    let fixture = daemon_fixture("bootstrap-sub-close").expect("fixture should start");
    let created = fixture
        .client
        .create_session(
            BackendKind::Native,
            CreateSessionSpec { title: Some("shell".to_string()), ..CreateSessionSpec::default() },
        )
        .await
        .expect("create_session should succeed");
    let mut subscription = fixture
        .client
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

    fixture.shutdown().await.expect("fixture should stop cleanly");
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_smoke_streams_live_pane_surface_updates() {
    let fixture = daemon_fixture("bootstrap-pane-sub").expect("fixture should start");
    let created = fixture
        .client
        .create_session(
            BackendKind::Native,
            CreateSessionSpec { title: Some("shell".to_string()), launch: Some(cat_launch_spec()) },
        )
        .await
        .expect("create_session should succeed");
    let topology = fixture
        .client
        .topology_snapshot(created.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let pane_id = topology.tabs[0].focused_pane.expect("focused pane should exist");
    wait_for_screen_line(&fixture, created.session.session_id, pane_id, "ready").await;
    let mut subscription = fixture
        .client
        .open_subscription(created.session.session_id, SubscriptionSpec::PaneSurface { pane_id })
        .await
        .expect("subscription should open");

    let initial = subscription.recv().await.expect("recv should succeed").expect("event");
    let initial = match initial {
        SubscriptionEvent::ScreenDelta(delta) => delta,
        other => panic!("unexpected initial event: {other:?}"),
    };
    let dispatch = fixture
        .client
        .dispatch(
            created.session.session_id,
            MuxCommand::SendInput(SendInputSpec {
                pane_id,
                data: "hello from pane stream\r".to_string(),
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

    assert!(!dispatch.changed);
    assert!(initial.full_replace.is_some());
    assert!(updated.to_sequence > updated.from_sequence);
    assert!(
        patch.line_updates.iter().any(|line| line.line.text.contains("hello from pane stream"))
    );
    assert!(updated.full_replace.is_none());

    fixture.shutdown().await.expect("fixture should stop cleanly");
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_smoke_roundtrips_live_pty_io() {
    let fixture = daemon_fixture("bootstrap-pty-smoke").expect("fixture should start");
    let created = fixture
        .client
        .create_session(
            BackendKind::Native,
            CreateSessionSpec { title: Some("shell".to_string()), launch: Some(cat_launch_spec()) },
        )
        .await
        .expect("create_session should succeed");
    let topology = fixture
        .client
        .topology_snapshot(created.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let pane_id = topology.tabs[0].focused_pane.expect("focused pane should exist");

    wait_for_screen_line(&fixture, created.session.session_id, pane_id, "ready").await;
    let before = fixture
        .client
        .screen_snapshot(created.session.session_id, pane_id)
        .await
        .expect("screen_snapshot should succeed");
    let dispatch = fixture
        .client
        .dispatch(
            created.session.session_id,
            MuxCommand::SendInput(SendInputSpec {
                pane_id,
                data: "hello from smoke\r".to_string(),
            }),
        )
        .await
        .expect("dispatch should succeed");

    assert!(!dispatch.changed);
    wait_for_screen_line(&fixture, created.session.session_id, pane_id, "hello from smoke").await;
    let delta = fixture
        .client
        .screen_delta(created.session.session_id, pane_id, before.sequence)
        .await
        .expect("screen_delta should succeed");
    let patch = delta.patch.expect("delta patch should exist");

    assert_eq!(delta.pane_id, pane_id);
    assert_eq!(delta.from_sequence, before.sequence);
    assert!(delta.to_sequence > before.sequence);
    assert!(patch.line_updates.iter().any(|line| line.line.text.contains("hello from smoke")));
    assert!(delta.full_replace.is_none());

    fixture.shutdown().await.expect("fixture should stop cleanly");
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_smoke_discovers_and_imports_tmux_session() {
    let socket_name = unique_tmux_socket_name("bootstrap-tmux");
    let session_name = unique_tmux_session_name("workspace");
    let _tmux =
        TmuxServerGuard::spawn(&socket_name, &session_name).expect("tmux test server should start");
    let fixture =
        daemon_fixture_with_state("bootstrap-tmux-import", tmux_daemon_state(&socket_name))
            .expect("fixture should start");

    let discovered = fixture
        .client
        .discover_sessions(BackendKind::Tmux)
        .await
        .expect("discover_sessions should succeed");
    assert_eq!(discovered.sessions.len(), 1);
    let candidate = discovered.sessions[0].clone();
    let imported = fixture
        .client
        .import_session(candidate.route.clone(), candidate.title.clone())
        .await
        .expect("import_session should succeed");
    let imported_again = fixture
        .client
        .import_session(candidate.route.clone(), candidate.title.clone())
        .await
        .expect("second import should be idempotent");
    let listed = fixture.client.list_sessions().await.expect("list_sessions should succeed");
    let topology = fixture
        .client
        .topology_snapshot(imported.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let focused_tab = topology.focused_tab.expect("focused tab should exist");
    let focused_pane = topology
        .tabs
        .iter()
        .find(|tab| tab.tab_id == focused_tab)
        .and_then(|tab| tab.focused_pane)
        .expect("focused pane should exist");
    let screen = fixture
        .client
        .screen_snapshot(imported.session.session_id, focused_pane)
        .await
        .expect("screen_snapshot should succeed");
    let delta = fixture
        .client
        .screen_delta(imported.session.session_id, focused_pane, screen.sequence)
        .await
        .expect("screen_delta should succeed");
    let dispatch_error = fixture
        .client
        .dispatch(
            imported.session.session_id,
            MuxCommand::NewTab(NewTabSpec { title: Some("forbidden".to_string()) }),
        )
        .await
        .expect_err("tmux imported routes should be observe-only");

    assert_eq!(imported.session.route.backend, BackendKind::Tmux);
    assert_eq!(imported.session.session_id, imported_again.session.session_id);
    assert_eq!(listed.sessions.len(), 1);
    assert_eq!(topology.backend_kind, BackendKind::Tmux);
    assert_eq!(topology.tabs.len(), 2);
    assert_eq!(screen.source, ProjectionSource::TmuxCapturePane);
    assert!(screen.surface.lines.iter().any(|line| line.text.contains("hello from tmux")));
    assert!(delta.patch.is_none());
    assert!(delta.full_replace.is_none());
    assert_eq!(dispatch_error.code, "backend_unsupported");

    fixture.shutdown().await.expect("fixture should stop cleanly");
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_smoke_streams_tmux_topology_updates() {
    let socket_name = unique_tmux_socket_name("bootstrap-tmux-topology");
    let session_name = unique_tmux_session_name("workspace");
    let _tmux =
        TmuxServerGuard::spawn(&socket_name, &session_name).expect("tmux test server should start");
    let fixture =
        daemon_fixture_with_state("bootstrap-tmux-topology-sub", tmux_daemon_state(&socket_name))
            .expect("fixture should start");

    let discovered = fixture
        .client
        .discover_sessions(BackendKind::Tmux)
        .await
        .expect("discover_sessions should succeed");
    let imported = fixture
        .client
        .import_session(discovered.sessions[0].route.clone(), discovered.sessions[0].title.clone())
        .await
        .expect("import_session should succeed");
    let mut subscription = fixture
        .client
        .open_subscription(imported.session.session_id, SubscriptionSpec::SessionTopology)
        .await
        .expect("subscription should open");

    let initial = subscription.recv().await.expect("recv should succeed").expect("event");
    let initial = match initial {
        SubscriptionEvent::TopologySnapshot(snapshot) => snapshot,
        other => panic!("unexpected initial event: {other:?}"),
    };

    run_tmux(
        &socket_name,
        &[
            "new-window",
            "-d",
            "-t",
            &session_name,
            "-n",
            "metrics",
            "sh",
            "-lc",
            "printf 'metrics ready\\n'; exec cat",
        ],
    )
    .expect("tmux new-window should succeed");

    let updated = subscription.recv().await.expect("recv should succeed").expect("event");
    let updated = match updated {
        SubscriptionEvent::TopologySnapshot(snapshot) => snapshot,
        other => panic!("unexpected topology event: {other:?}"),
    };

    assert_eq!(initial.tabs.len(), 2);
    assert_eq!(updated.tabs.len(), 3);

    fixture.shutdown().await.expect("fixture should stop cleanly");
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_smoke_streams_tmux_pane_surface_updates() {
    let socket_name = unique_tmux_socket_name("bootstrap-tmux-pane");
    let session_name = unique_tmux_session_name("workspace");
    let _tmux =
        TmuxServerGuard::spawn(&socket_name, &session_name).expect("tmux test server should start");
    let fixture =
        daemon_fixture_with_state("bootstrap-tmux-pane-sub", tmux_daemon_state(&socket_name))
            .expect("fixture should start");

    let discovered = fixture
        .client
        .discover_sessions(BackendKind::Tmux)
        .await
        .expect("discover_sessions should succeed");
    let imported = fixture
        .client
        .import_session(discovered.sessions[0].route.clone(), discovered.sessions[0].title.clone())
        .await
        .expect("import_session should succeed");
    let topology = fixture
        .client
        .topology_snapshot(imported.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let pane_id = topology.tabs[0].focused_pane.expect("focused pane should exist");
    let mut subscription = fixture
        .client
        .open_subscription(imported.session.session_id, SubscriptionSpec::PaneSurface { pane_id })
        .await
        .expect("subscription should open");

    let initial = subscription.recv().await.expect("recv should succeed").expect("event");
    let initial = match initial {
        SubscriptionEvent::ScreenDelta(delta) => delta,
        other => panic!("unexpected initial event: {other:?}"),
    };

    run_tmux(
        &socket_name,
        &["send-keys", "-t", &format!("{session_name}:0.0"), "hello from tmux subscription", "C-m"],
    )
    .expect("tmux send-keys should succeed");

    let updated = subscription.recv().await.expect("recv should succeed").expect("event");
    let updated = match updated {
        SubscriptionEvent::ScreenDelta(delta) => delta,
        other => panic!("unexpected pane event: {other:?}"),
    };
    let patch = updated.patch.expect("delta patch should exist");

    assert!(initial.full_replace.is_some());
    assert!(updated.to_sequence > updated.from_sequence);
    assert!(
        patch
            .line_updates
            .iter()
            .any(|line| line.line.text.contains("hello from tmux subscription"))
    );
    assert!(updated.full_replace.is_none());

    fixture.shutdown().await.expect("fixture should stop cleanly");
}

#[cfg(unix)]
fn cat_launch_spec() -> ShellLaunchSpec {
    ShellLaunchSpec::new("/bin/sh").with_args(["-lc", "printf 'ready\\n'; exec cat"])
}

#[cfg(unix)]
async fn wait_for_screen_line(
    fixture: &terminal_testing::DaemonFixture,
    session_id: terminal_domain::SessionId,
    pane_id: terminal_domain::PaneId,
    needle: &str,
) {
    for _ in 0..40 {
        let screen = fixture
            .client
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

#[cfg(unix)]
fn tmux_daemon_state(socket_name: &str) -> TerminalDaemonState {
    TerminalDaemonState::new(BackendCatalog::new([
        Arc::new(NativeBackend::default()) as Arc<dyn MuxBackendPort>,
        Arc::new(TmuxBackend::with_socket_name(socket_name)) as Arc<dyn MuxBackendPort>,
        Arc::new(ZellijBackend) as Arc<dyn MuxBackendPort>,
    ]))
}

#[cfg(unix)]
fn unique_tmux_socket_name(label: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("terminal-platform-{label}-{}-{nanos}", std::process::id())
}

#[cfg(unix)]
fn unique_tmux_session_name(label: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("{label}-{}-{nanos}", std::process::id())
}

#[cfg(unix)]
struct TmuxServerGuard {
    socket_name: String,
}

#[cfg(unix)]
impl TmuxServerGuard {
    fn spawn(socket_name: &str, session_name: &str) -> Result<Self, String> {
        run_tmux(
            socket_name,
            &[
                "new-session",
                "-d",
                "-s",
                session_name,
                "sh",
                "-lc",
                "printf 'hello from tmux\\n'; exec cat",
            ],
        )?;
        run_tmux(
            socket_name,
            &[
                "new-window",
                "-d",
                "-t",
                session_name,
                "-n",
                "logs",
                "sh",
                "-lc",
                "printf 'logs ready\\n'; exec cat",
            ],
        )?;

        Ok(Self { socket_name: socket_name.to_string() })
    }
}

#[cfg(unix)]
impl Drop for TmuxServerGuard {
    fn drop(&mut self) {
        let _ = run_tmux(&self.socket_name, &["kill-server"]);
    }
}

#[cfg(unix)]
fn run_tmux(socket_name: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new("tmux")
        .arg("-L")
        .arg(socket_name)
        .args(args)
        .output()
        .map_err(|error| format!("failed to spawn tmux: {error}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }

    String::from_utf8(output.stdout).map_err(|error| format!("invalid tmux utf8 output: {error}"))
}
