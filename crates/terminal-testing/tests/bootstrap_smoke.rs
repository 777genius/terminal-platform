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
    SplitPaneSpec, SubscriptionSpec,
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
use terminal_domain::DegradedModeReason;
#[cfg(unix)]
use terminal_domain::PaneId;
#[cfg(unix)]
use terminal_mux_domain::{PaneTreeNode, SplitDirection};
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
async fn bootstrap_smoke_reports_dynamic_backend_capabilities() {
    let fixture = daemon_fixture("bootstrap-backend-capabilities").expect("fixture should start");

    let native = fixture
        .client
        .backend_capabilities(BackendKind::Native)
        .await
        .expect("native capabilities should succeed");
    let tmux = fixture
        .client
        .backend_capabilities(BackendKind::Tmux)
        .await
        .expect("tmux capabilities should succeed");
    let zellij = fixture
        .client
        .backend_capabilities(BackendKind::Zellij)
        .await
        .expect("zellij capabilities should succeed");

    assert_eq!(native.backend, BackendKind::Native);
    assert!(native.capabilities.tiled_panes);
    assert!(native.capabilities.tab_create);
    assert!(native.capabilities.tab_close);
    assert!(native.capabilities.tab_focus);
    assert!(native.capabilities.tab_rename);
    assert!(!native.capabilities.pane_split);
    assert!(native.capabilities.pane_focus);
    assert!(native.capabilities.pane_input_write);
    assert!(native.capabilities.rendered_viewport_stream);
    assert_eq!(tmux.backend, BackendKind::Tmux);
    assert!(tmux.capabilities.read_only_client_mode);
    assert!(tmux.capabilities.tab_create);
    assert!(tmux.capabilities.tab_close);
    assert!(tmux.capabilities.tab_focus);
    assert!(tmux.capabilities.tab_rename);
    assert!(tmux.capabilities.pane_split);
    assert!(tmux.capabilities.pane_focus);
    assert!(tmux.capabilities.pane_input_write);
    assert!(tmux.capabilities.rendered_viewport_stream);
    assert_eq!(zellij.backend, BackendKind::Zellij);
    assert!(zellij.capabilities.read_only_client_mode);
    assert!(!zellij.capabilities.tab_create);
    assert!(!zellij.capabilities.tab_close);
    assert!(!zellij.capabilities.tab_focus);
    assert!(!zellij.capabilities.pane_split);
    assert!(!zellij.capabilities.pane_focus);
    assert!(!zellij.capabilities.rendered_viewport_stream);

    fixture.shutdown().await.expect("fixture should stop cleanly");
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
    let mut updated = match updated {
        SubscriptionEvent::TopologySnapshot(snapshot) => snapshot,
        other => panic!("unexpected topology event: {other:?}"),
    };
    while updated.tabs.len() != 2 {
        let next = subscription.recv().await.expect("recv should succeed").expect("event");
        updated = match next {
            SubscriptionEvent::TopologySnapshot(snapshot) => snapshot,
            other => panic!("unexpected topology event: {other:?}"),
        };
    }

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
    assert_ne!(updated.to_sequence, updated.from_sequence);
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
    let rename = fixture
        .client
        .dispatch(
            imported.session.session_id,
            MuxCommand::RenameTab { tab_id: focused_tab, title: "workspace-renamed".to_string() },
        )
        .await
        .expect("rename tab should succeed");
    let topology_after_rename = fixture
        .client
        .topology_snapshot(imported.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let send_input = fixture
        .client
        .dispatch(
            imported.session.session_id,
            MuxCommand::SendInput(SendInputSpec {
                pane_id: focused_pane,
                data: "hello from tmux dispatch\r".to_string(),
            }),
        )
        .await
        .expect("send input should succeed");
    wait_for_screen_line(
        &fixture,
        imported.session.session_id,
        focused_pane,
        "hello from tmux dispatch",
    )
    .await;
    let screen_after_input = fixture
        .client
        .screen_snapshot(imported.session.session_id, focused_pane)
        .await
        .expect("screen_snapshot should succeed");
    let secondary_tab_id = topology
        .tabs
        .iter()
        .find(|tab| tab.tab_id != focused_tab)
        .map(|tab| tab.tab_id)
        .expect("secondary tab should exist");
    let close_tab = fixture
        .client
        .dispatch(imported.session.session_id, MuxCommand::CloseTab { tab_id: secondary_tab_id })
        .await
        .expect("close tab should succeed");
    let topology_after_close = fixture
        .client
        .topology_snapshot(imported.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let close_last_error = fixture
        .client
        .dispatch(imported.session.session_id, MuxCommand::CloseTab { tab_id: focused_tab })
        .await
        .expect_err("closing the last tmux tab should be rejected");
    let delta = fixture
        .client
        .screen_delta(imported.session.session_id, focused_pane, screen.sequence)
        .await
        .expect("screen_delta should succeed");
    let dispatch_error = fixture
        .client
        .dispatch(imported.session.session_id, MuxCommand::SaveSession)
        .await
        .expect_err("tmux imported routes should reject unsupported control paths");

    assert_eq!(imported.session.route.backend, BackendKind::Tmux);
    assert_eq!(imported.session.session_id, imported_again.session.session_id);
    assert_eq!(listed.sessions.len(), 1);
    assert_eq!(topology.backend_kind, BackendKind::Tmux);
    assert_eq!(topology.tabs.len(), 2);
    assert!(rename.changed);
    assert!(
        topology_after_rename
            .tabs
            .iter()
            .any(|tab| tab.tab_id == focused_tab
                && tab.title.as_deref() == Some("workspace-renamed"))
    );
    assert!(send_input.changed);
    assert!(close_tab.changed);
    assert_eq!(topology_after_close.tabs.len(), 1);
    assert_eq!(screen.source, ProjectionSource::TmuxCapturePane);
    assert!(screen.surface.lines.iter().any(|line| line.text.contains("hello from tmux")));
    assert!(
        screen_after_input
            .surface
            .lines
            .iter()
            .any(|line| line.text.contains("hello from tmux dispatch"))
    );
    assert!(delta.patch.is_none());
    assert!(delta.full_replace.is_some());
    assert_eq!(dispatch_error.code, "backend_unsupported");
    assert_eq!(dispatch_error.degraded_reason, Some(DegradedModeReason::UnsupportedByBackend));
    assert_eq!(close_last_error.code, "backend_unsupported");
    assert_eq!(close_last_error.degraded_reason, Some(DegradedModeReason::UnsupportedByBackend));

    fixture.shutdown().await.expect("fixture should stop cleanly");
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_smoke_controls_tmux_tab_lifecycle_via_dispatch() {
    let socket_name = unique_tmux_socket_name("bootstrap-tmux-tabs");
    let session_name = unique_tmux_session_name("workspace");
    let _tmux =
        TmuxServerGuard::spawn(&socket_name, &session_name).expect("tmux test server should start");
    let fixture =
        daemon_fixture_with_state("bootstrap-tmux-tab-control", tmux_daemon_state(&socket_name))
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
    let initial = fixture
        .client
        .topology_snapshot(imported.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let initial_focused_tab = initial.focused_tab.expect("focused tab should exist");

    let created = fixture
        .client
        .dispatch(
            imported.session.session_id,
            MuxCommand::NewTab(NewTabSpec { title: Some("metrics".to_string()) }),
        )
        .await
        .expect("new tab should succeed");
    let after_create = fixture
        .client
        .topology_snapshot(imported.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let metrics_tab = after_create
        .tabs
        .iter()
        .find(|tab| tab.title.as_deref() == Some("metrics"))
        .map(|tab| tab.tab_id)
        .expect("created tab should exist");

    let focused = fixture
        .client
        .dispatch(imported.session.session_id, MuxCommand::FocusTab { tab_id: initial_focused_tab })
        .await
        .expect("focus tab should succeed");
    let after_focus = fixture
        .client
        .topology_snapshot(imported.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let closed = fixture
        .client
        .dispatch(imported.session.session_id, MuxCommand::CloseTab { tab_id: metrics_tab })
        .await
        .expect("close tab should succeed");
    let after_close = fixture
        .client
        .topology_snapshot(imported.session.session_id)
        .await
        .expect("topology_snapshot should succeed");

    assert!(created.changed);
    assert_eq!(after_create.tabs.len(), 3);
    assert_eq!(after_create.focused_tab, Some(metrics_tab));
    assert!(focused.changed);
    assert_eq!(after_focus.focused_tab, Some(initial_focused_tab));
    assert!(closed.changed);
    assert_eq!(after_close.tabs.len(), 2);
    assert!(after_close.tabs.iter().all(|tab| tab.tab_id != metrics_tab));

    fixture.shutdown().await.expect("fixture should stop cleanly");
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_smoke_controls_tmux_pane_lifecycle_via_dispatch() {
    let socket_name = unique_tmux_socket_name("bootstrap-tmux-panes");
    let session_name = unique_tmux_session_name("workspace");
    let _tmux =
        TmuxServerGuard::spawn(&socket_name, &session_name).expect("tmux test server should start");
    let fixture =
        daemon_fixture_with_state("bootstrap-tmux-pane-control", tmux_daemon_state(&socket_name))
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
    let initial = fixture
        .client
        .topology_snapshot(imported.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let focused_tab = initial.focused_tab.expect("focused tab should exist");
    let focused_tab_snapshot = initial
        .tabs
        .iter()
        .find(|tab| tab.tab_id == focused_tab)
        .expect("focused tab snapshot should exist");
    let focused_pane = focused_tab_snapshot.focused_pane.expect("focused pane should exist");
    let initial_panes = collect_pane_ids(&focused_tab_snapshot.root);

    let split = fixture
        .client
        .dispatch(
            imported.session.session_id,
            MuxCommand::SplitPane(SplitPaneSpec {
                pane_id: focused_pane,
                direction: SplitDirection::Vertical,
            }),
        )
        .await
        .expect("split pane should succeed");
    let after_split = fixture
        .client
        .topology_snapshot(imported.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let split_tab = after_split
        .tabs
        .iter()
        .find(|tab| tab.tab_id == focused_tab)
        .expect("split tab snapshot should exist");
    let split_panes = collect_pane_ids(&split_tab.root);
    let new_pane = split_panes
        .iter()
        .copied()
        .find(|pane_id| !initial_panes.contains(pane_id))
        .expect("new pane should exist after split");

    let focus = fixture
        .client
        .dispatch(imported.session.session_id, MuxCommand::FocusPane { pane_id: focused_pane })
        .await
        .expect("focus pane should succeed");
    let after_focus = fixture
        .client
        .topology_snapshot(imported.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let focused_tab_after_focus = after_focus
        .tabs
        .iter()
        .find(|tab| tab.tab_id == focused_tab)
        .expect("focused tab snapshot should exist");

    assert!(split.changed);
    assert_eq!(split_panes.len(), initial_panes.len() + 1);
    assert_eq!(split_tab.focused_pane, Some(new_pane));
    assert!(focus.changed);
    assert_eq!(focused_tab_after_focus.focused_pane, Some(focused_pane));

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
    let mut updated = match updated {
        SubscriptionEvent::TopologySnapshot(snapshot) => snapshot,
        other => panic!("unexpected topology event: {other:?}"),
    };
    while updated.tabs.len() != 3 {
        let next = subscription.recv().await.expect("recv should succeed").expect("event");
        updated = match next {
            SubscriptionEvent::TopologySnapshot(snapshot) => snapshot,
            other => panic!("unexpected topology event: {other:?}"),
        };
    }

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

    let dispatch = fixture
        .client
        .dispatch(
            imported.session.session_id,
            MuxCommand::SendInput(SendInputSpec {
                pane_id,
                data: "hello from tmux subscription\r".to_string(),
            }),
        )
        .await
        .expect("send input should succeed");

    let updated = loop {
        let next = subscription.recv().await.expect("recv should succeed").expect("event");
        let next = match next {
            SubscriptionEvent::ScreenDelta(delta) => delta,
            other => panic!("unexpected pane event: {other:?}"),
        };
        let Some(patch) = next.patch.as_ref() else {
            continue;
        };
        if patch
            .line_updates
            .iter()
            .any(|line| line.line.text.contains("hello from tmux subscription"))
        {
            break next;
        }
    };
    let patch = updated.patch.expect("delta patch should exist");

    assert!(dispatch.changed);
    assert!(initial.full_replace.is_some());
    assert_ne!(updated.to_sequence, updated.from_sequence);
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
#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_smoke_discovers_zellij_session_and_rejects_import_on_legacy_surface() {
    let session_name = unique_zellij_session_name("workspace");
    let _zellij = ZellijSessionGuard::spawn(&session_name).expect("zellij session should start");
    let fixture = daemon_fixture("bootstrap-zellij-discover").expect("fixture should start");

    let discovered = tokio::time::timeout(
        Duration::from_secs(10),
        fixture.client.discover_sessions(BackendKind::Zellij),
    )
    .await
    .expect("discover_sessions should not hang")
    .expect("discover_sessions should succeed");
    let candidate = discovered
        .sessions
        .iter()
        .find(|session| session.title.as_deref() == Some(session_name.as_str()))
        .cloned()
        .expect("created zellij session should be discoverable");
    let error = tokio::time::timeout(
        Duration::from_secs(10),
        fixture.client.import_session(candidate.route.clone(), candidate.title.clone()),
    )
    .await
    .expect("import_session should not hang")
    .expect_err("legacy local zellij surface should reject imported attach");
    let listed = fixture.client.list_sessions().await.expect("list_sessions should succeed");

    assert_eq!(candidate.route.backend, BackendKind::Zellij);
    assert_eq!(error.code, "backend_unsupported");
    assert!(error.message.contains("zellij 0.43.1"));
    assert_eq!(error.degraded_reason, Some(DegradedModeReason::MissingCapability));
    assert!(listed.sessions.is_empty());

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
fn collect_pane_ids(root: &PaneTreeNode) -> Vec<PaneId> {
    let mut pane_ids = Vec::new();
    collect_pane_ids_inner(root, &mut pane_ids);
    pane_ids
}

#[cfg(unix)]
fn collect_pane_ids_inner(root: &PaneTreeNode, pane_ids: &mut Vec<PaneId>) {
    match root {
        PaneTreeNode::Leaf { pane_id } => pane_ids.push(*pane_id),
        PaneTreeNode::Split(split) => {
            collect_pane_ids_inner(&split.first, pane_ids);
            collect_pane_ids_inner(&split.second, pane_ids);
        }
    }
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

#[cfg(unix)]
fn unique_zellij_session_name(label: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let entropy = (nanos & 0xffff_ffff) as u64;
    format!("tp-{}-{:x}", label.chars().take(8).collect::<String>(), entropy)
}

#[cfg(unix)]
struct ZellijSessionGuard {
    session_name: String,
}

#[cfg(unix)]
impl ZellijSessionGuard {
    fn spawn(session_name: &str) -> Result<Self, String> {
        let output = Command::new("zellij")
            .args(["--session", session_name, "--new-session-with-layout", "default"])
            .output()
            .map_err(|error| format!("failed to spawn zellij: {error}"))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stderr.contains("could not get terminal attribute: ENODEV") {
                return Err(stderr.trim().to_string());
            }
        }
        wait_for_zellij_session(session_name)?;
        Ok(Self { session_name: session_name.to_string() })
    }
}

#[cfg(unix)]
impl Drop for ZellijSessionGuard {
    fn drop(&mut self) {
        let _ = run_zellij(&["kill-session", &self.session_name]);
    }
}

#[cfg(unix)]
fn run_zellij(args: &[&str]) -> Result<String, String> {
    let output = Command::new("zellij")
        .args(args)
        .output()
        .map_err(|error| format!("failed to spawn zellij: {error}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }

    String::from_utf8(output.stdout).map_err(|error| format!("invalid zellij utf8 output: {error}"))
}

#[cfg(unix)]
fn wait_for_zellij_session(session_name: &str) -> Result<(), String> {
    for _ in 0..40 {
        let sessions = run_zellij(&["list-sessions", "--short", "--no-formatting"])?;
        if sessions.lines().map(str::trim).any(|line| line == session_name) {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(50));
    }

    Err(format!("zellij session never appeared: {session_name}"))
}
