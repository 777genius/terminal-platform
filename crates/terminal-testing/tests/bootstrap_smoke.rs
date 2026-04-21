use std::{
    thread,
    time::{Duration, Instant},
};

#[cfg(unix)]
use std::{
    fs,
    process::Command,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(unix)]
use terminal_application::BackendCatalog;
use terminal_backend_api::{
    CreateSessionSpec, MuxBackendPort, MuxCommand, NewTabSpec, OverrideLayoutSpec, ResizePaneSpec,
    SendInputSpec, ShellLaunchSpec, SplitPaneSpec, SubscriptionSpec,
};
#[cfg(unix)]
use terminal_backend_native::NativeBackend;
#[cfg(unix)]
use terminal_backend_tmux::TmuxBackend;
#[cfg(unix)]
use terminal_backend_zellij::ZellijBackend;
#[cfg(unix)]
use terminal_daemon::TerminalDaemonState;
use terminal_domain::{
    BackendKind, CURRENT_BINARY_VERSION, CURRENT_PROTOCOL_MAJOR, CURRENT_PROTOCOL_MINOR,
};
#[cfg(unix)]
use terminal_domain::{
    CURRENT_SAVED_SESSION_FORMAT_VERSION, SavedSessionCompatibilityStatus, SavedSessionManifest,
    SessionId, local_native_route,
};
#[cfg(any(unix, windows))]
use terminal_domain::{DegradedModeReason, PaneId, TabId};
#[cfg(any(unix, windows))]
use terminal_mux_domain::PaneTreeNode;
#[cfg(unix)]
use terminal_mux_domain::{PaneSplit, SplitDirection, TabSnapshot};
#[cfg(unix)]
use terminal_persistence::SqliteSessionStore;
#[cfg(any(unix, windows))]
use terminal_projection::{ProjectionSource, ScreenDelta, ScreenSnapshot, TopologySnapshot};
use terminal_protocol::{DaemonPhase, SubscriptionEvent};
use terminal_testing::{
    ZellijSessionGuard, ZellijTestLock, daemon_fixture, daemon_fixture_with_state, daemon_state,
    echo_shell_launch_spec, isolated_daemon_state, unique_sqlite_path, unique_zellij_session_name,
};
use tokio::time::{sleep, timeout};

#[test]
fn bootstrap_smoke_exposes_empty_daemon_state() {
    let daemon = daemon_state();
    let handshake = daemon.handshake();

    assert_eq!(handshake.protocol_version.major, 0);
    assert_eq!(handshake.protocol_version.minor, 1);
    assert_eq!(handshake.daemon_phase, DaemonPhase::Ready);
    assert_eq!(
        handshake.available_backends,
        vec![BackendKind::Native, BackendKind::Tmux, BackendKind::Zellij]
    );
    assert!(handshake.capabilities.request_reply);
    assert!(handshake.capabilities.topology_subscriptions);
    assert!(handshake.capabilities.pane_subscriptions);
    assert!(handshake.capabilities.backend_discovery);
    assert!(handshake.capabilities.backend_capability_queries);
    assert!(handshake.capabilities.saved_sessions);
    assert!(handshake.capabilities.session_restore);
    assert!(handshake.capabilities.degraded_error_reasons);
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
    assert_eq!(tmux.backend, BackendKind::Tmux);
    assert!(tmux.capabilities.read_only_client_mode);
    assert!(tmux.capabilities.split_resize);
    assert!(tmux.capabilities.tab_create);
    assert!(tmux.capabilities.tab_close);
    assert!(tmux.capabilities.tab_focus);
    assert!(tmux.capabilities.tab_rename);
    assert!(tmux.capabilities.pane_split);
    assert!(tmux.capabilities.pane_close);
    assert!(tmux.capabilities.pane_focus);
    assert!(tmux.capabilities.pane_input_write);
    assert!(tmux.capabilities.rendered_viewport_stream);
    assert_eq!(zellij.backend, BackendKind::Zellij);
    assert!(zellij.capabilities.read_only_client_mode);
    assert!(!zellij.capabilities.split_resize);
    assert!(!zellij.capabilities.pane_split);
    if zellij.capabilities.rendered_viewport_snapshot {
        assert!(zellij.capabilities.tiled_panes);
        assert!(zellij.capabilities.tab_create);
        assert!(zellij.capabilities.tab_close);
        assert!(zellij.capabilities.tab_focus);
        assert!(zellij.capabilities.tab_rename);
        assert!(zellij.capabilities.session_scoped_tab_refs);
        assert!(zellij.capabilities.session_scoped_pane_refs);
        assert!(zellij.capabilities.pane_close);
        assert!(zellij.capabilities.pane_focus);
        assert!(zellij.capabilities.pane_input_write);
        assert!(zellij.capabilities.pane_paste_write);
        assert!(zellij.capabilities.rendered_viewport_stream);
        assert!(zellij.capabilities.plugin_panes);
        assert!(zellij.capabilities.advisory_metadata_subscriptions);
        assert!(!zellij.capabilities.floating_panes);
        assert!(!zellij.capabilities.rendered_scrollback_snapshot);
    } else {
        assert!(!zellij.capabilities.tab_create);
        assert!(!zellij.capabilities.tab_close);
        assert!(!zellij.capabilities.tab_focus);
        assert!(!zellij.capabilities.tab_rename);
        assert!(!zellij.capabilities.tiled_panes);
        assert!(!zellij.capabilities.session_scoped_tab_refs);
        assert!(!zellij.capabilities.session_scoped_pane_refs);
        assert!(!zellij.capabilities.pane_close);
        assert!(!zellij.capabilities.pane_focus);
        assert!(!zellij.capabilities.pane_input_write);
        assert!(!zellij.capabilities.pane_paste_write);
        assert!(!zellij.capabilities.rendered_viewport_stream);
        assert!(!zellij.capabilities.plugin_panes);
        assert!(!zellij.capabilities.advisory_metadata_subscriptions);
    }

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
    let handshake_assessment =
        fixture.client.handshake_assessment().await.expect("handshake_assessment should succeed");
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
    assert_eq!(handshake.daemon_phase, DaemonPhase::Ready);
    assert!(handshake.capabilities.request_reply);
    assert!(handshake.capabilities.saved_sessions);
    assert!(handshake_assessment.can_use);
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

    let initial = must_recv_subscription_event(&mut subscription).await;
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
    let updated = must_recv_subscription_event(&mut subscription).await;
    let mut updated = match updated {
        SubscriptionEvent::TopologySnapshot(snapshot) => snapshot,
        other => panic!("unexpected topology event: {other:?}"),
    };
    while updated.tabs.len() != 2 {
        let next = must_recv_subscription_event(&mut subscription).await;
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

    let initial = must_recv_subscription_event(&mut subscription).await;
    match initial {
        SubscriptionEvent::TopologySnapshot(_) => {}
        other => panic!("unexpected initial event: {other:?}"),
    }
    subscription.close().await.expect("close should succeed");
    assert!(recv_subscription_event(&mut subscription).await.is_none());

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

    let initial = must_recv_subscription_event(&mut subscription).await;
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
                data: submitted_input("hello from pane stream"),
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
                data: submitted_input("hello from smoke"),
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
async fn bootstrap_smoke_streams_surface_updates_for_all_native_panes_after_resize() {
    let fixture = daemon_fixture("bootstrap-native-pane-resize-sub").expect("fixture should start");
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
    let original_pane = topology.tabs[0].focused_pane.expect("focused pane should exist");

    wait_for_screen_line(&fixture, created.session.session_id, original_pane, "ready").await;
    fixture
        .client
        .dispatch(
            created.session.session_id,
            MuxCommand::SplitPane(SplitPaneSpec {
                pane_id: original_pane,
                direction: SplitDirection::Vertical,
            }),
        )
        .await
        .expect("split pane should succeed");
    let after_split = fixture
        .client
        .topology_snapshot(created.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let pane_ids = collect_pane_ids(&after_split.tabs[0].root);
    let resized_pane = pane_ids
        .iter()
        .copied()
        .find(|candidate| *candidate != original_pane)
        .expect("new pane should exist");
    wait_for_screen_line(&fixture, created.session.session_id, resized_pane, "ready").await;

    let mut original_subscription = fixture
        .client
        .open_subscription(
            created.session.session_id,
            SubscriptionSpec::PaneSurface { pane_id: original_pane },
        )
        .await
        .expect("original subscription should open");
    let mut resized_subscription = fixture
        .client
        .open_subscription(
            created.session.session_id,
            SubscriptionSpec::PaneSurface { pane_id: resized_pane },
        )
        .await
        .expect("resized subscription should open");

    let original_initial = match must_recv_subscription_event(&mut original_subscription).await {
        SubscriptionEvent::ScreenDelta(delta) => delta,
        other => panic!("unexpected original initial event: {other:?}"),
    };
    let resized_initial = match must_recv_subscription_event(&mut resized_subscription).await {
        SubscriptionEvent::ScreenDelta(delta) => delta,
        other => panic!("unexpected resized initial event: {other:?}"),
    };

    let original_before = fixture
        .client
        .screen_snapshot(created.session.session_id, original_pane)
        .await
        .expect("screen_snapshot should succeed");
    let resized_before = fixture
        .client
        .screen_snapshot(created.session.session_id, resized_pane)
        .await
        .expect("screen_snapshot should succeed");
    let total_cols = original_before.cols.saturating_add(resized_before.cols);
    let target_cols = resized_before.cols.saturating_add(10).min(total_cols.saturating_sub(1));
    let resize = fixture
        .client
        .dispatch(
            created.session.session_id,
            MuxCommand::ResizePane(ResizePaneSpec {
                pane_id: resized_pane,
                rows: resized_before.rows,
                cols: target_cols,
            }),
        )
        .await
        .expect("resize should succeed");

    let original_updated = match must_recv_subscription_event(&mut original_subscription).await {
        SubscriptionEvent::ScreenDelta(delta) => delta,
        other => panic!("unexpected original updated event: {other:?}"),
    };
    let resized_updated = match must_recv_subscription_event(&mut resized_subscription).await {
        SubscriptionEvent::ScreenDelta(delta) => delta,
        other => panic!("unexpected resized updated event: {other:?}"),
    };

    assert!(original_initial.full_replace.is_some());
    assert!(resized_initial.full_replace.is_some());
    assert!(resize.changed);
    assert_eq!(original_updated.pane_id, original_pane);
    assert_eq!(resized_updated.pane_id, resized_pane);
    assert!(original_updated.full_replace.is_some());
    assert!(resized_updated.full_replace.is_some());

    fixture.shutdown().await.expect("fixture should stop cleanly");
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_smoke_saves_native_session_snapshot_to_store() {
    let fixture = daemon_fixture("bootstrap-native-save").expect("fixture should start");
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
    fixture
        .client
        .dispatch(
            created.session.session_id,
            MuxCommand::SplitPane(SplitPaneSpec { pane_id, direction: SplitDirection::Vertical }),
        )
        .await
        .expect("split pane should succeed");
    let after_split = fixture
        .client
        .topology_snapshot(created.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let pane_ids = collect_pane_ids(&after_split.tabs[0].root);
    let new_pane = pane_ids
        .iter()
        .copied()
        .find(|candidate| *candidate != pane_id)
        .expect("new pane should exist");
    wait_for_screen_line(&fixture, created.session.session_id, new_pane, "ready").await;

    let save = fixture
        .client
        .dispatch(created.session.session_id, MuxCommand::SaveSession)
        .await
        .expect("save session should succeed");
    let store = SqliteSessionStore::open_default().expect("default store should open");
    let saved = store
        .load_native_session(created.session.session_id)
        .expect("load should succeed")
        .expect("saved session should exist");

    assert!(!save.changed);
    assert_eq!(saved.session_id, created.session.session_id);
    assert_eq!(saved.route.backend, BackendKind::Native);
    assert_eq!(saved.title.as_deref(), Some("shell"));
    assert_eq!(saved.topology.tabs.len(), 1);
    assert_eq!(collect_pane_ids(&saved.topology.tabs[0].root).len(), 2);
    assert_eq!(saved.screens.len(), 2);
    assert!(saved.launch.is_some());

    fixture.shutdown().await.expect("fixture should stop cleanly");
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_smoke_lists_and_loads_saved_native_sessions_via_daemon_api() {
    let fixture = daemon_fixture("bootstrap-native-saved-api").expect("fixture should start");
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
    fixture
        .client
        .dispatch(created.session.session_id, MuxCommand::SaveSession)
        .await
        .expect("save session should succeed");
    let saved =
        fixture.client.list_saved_sessions().await.expect("list_saved_sessions should succeed");
    let saved_summary = saved
        .sessions
        .iter()
        .find(|session| session.session_id == created.session.session_id)
        .expect("saved session should be listed");
    let loaded = fixture
        .client
        .saved_session(created.session.session_id)
        .await
        .expect("saved_session should succeed");

    assert_eq!(saved_summary.title.as_deref(), Some("shell"));
    assert_eq!(saved_summary.route.backend, BackendKind::Native);
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
    assert_eq!(loaded.session.topology.backend_kind, BackendKind::Native);
    assert_eq!(loaded.session.topology.tabs.len(), 1);
    assert_eq!(loaded.session.screens.len(), 1);
    assert_eq!(loaded.session.launch, Some(cat_launch_spec()));
    assert_eq!(loaded.session.manifest.binary_version, CURRENT_BINARY_VERSION);
    assert!(loaded.session.compatibility.can_restore);
    assert_eq!(loaded.session.compatibility.status, SavedSessionCompatibilityStatus::Compatible);
    assert!(loaded.session.restore_semantics.restores_focus_state);
    assert!(loaded.session.restore_semantics.restores_tab_titles);
    assert!(!loaded.session.restore_semantics.replays_saved_screen_buffers);
    assert!(!loaded.session.restore_semantics.preserves_process_state);

    fixture.shutdown().await.expect("fixture should stop cleanly");
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_smoke_deletes_saved_native_sessions_via_daemon_api() {
    let fixture = daemon_fixture("bootstrap-native-delete-saved").expect("fixture should start");
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
    fixture
        .client
        .dispatch(created.session.session_id, MuxCommand::SaveSession)
        .await
        .expect("save session should succeed");

    let deleted = fixture
        .client
        .delete_saved_session(created.session.session_id)
        .await
        .expect("delete_saved_session should succeed");
    let saved =
        fixture.client.list_saved_sessions().await.expect("list_saved_sessions should succeed");
    let lookup_error = fixture
        .client
        .saved_session(created.session.session_id)
        .await
        .expect_err("saved session should be gone after delete");

    assert_eq!(deleted.session_id, created.session.session_id);
    assert!(!saved.sessions.iter().any(|session| session.session_id == created.session.session_id));
    assert_eq!(lookup_error.code, "backend_not_found");

    fixture.shutdown().await.expect("fixture should stop cleanly");
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_smoke_restores_saved_native_session_via_daemon_api() {
    let fixture = daemon_fixture("bootstrap-native-restore-api").expect("fixture should start");
    let created = fixture
        .client
        .create_session(
            BackendKind::Native,
            CreateSessionSpec { title: Some("shell".to_string()), launch: Some(cat_launch_spec()) },
        )
        .await
        .expect("create_session should succeed");
    let initial = fixture
        .client
        .topology_snapshot(created.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let first_pane = initial.tabs[0].focused_pane.expect("focused pane should exist");

    wait_for_screen_line(&fixture, created.session.session_id, first_pane, "ready").await;
    fixture
        .client
        .dispatch(
            created.session.session_id,
            MuxCommand::SplitPane(SplitPaneSpec {
                pane_id: first_pane,
                direction: SplitDirection::Vertical,
            }),
        )
        .await
        .expect("split pane should succeed");
    fixture
        .client
        .dispatch(
            created.session.session_id,
            MuxCommand::NewTab(NewTabSpec { title: Some("logs".to_string()) }),
        )
        .await
        .expect("new tab should succeed");
    let before_save = fixture
        .client
        .topology_snapshot(created.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let second_tab_id = before_save.tabs[1].tab_id;
    fixture
        .client
        .dispatch(created.session.session_id, MuxCommand::FocusTab { tab_id: second_tab_id })
        .await
        .expect("focus tab should succeed");
    fixture
        .client
        .dispatch(created.session.session_id, MuxCommand::SaveSession)
        .await
        .expect("save session should succeed");

    let restored = fixture
        .client
        .restore_saved_session(created.session.session_id)
        .await
        .expect("restore_saved_session should succeed");
    let restored_topology = fixture
        .client
        .topology_snapshot(restored.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let first_restored_tab = &restored_topology.tabs[0];
    let second_restored_tab = &restored_topology.tabs[1];
    let restored_first_pane = first_restored_tab
        .focused_pane
        .or_else(|| collect_pane_ids(&first_restored_tab.root).into_iter().next())
        .expect("restored first tab should have a pane");
    let restored_second_pane = second_restored_tab
        .focused_pane
        .or_else(|| collect_pane_ids(&second_restored_tab.root).into_iter().next())
        .expect("restored second tab should have a pane");

    wait_for_screen_line(&fixture, restored.session.session_id, restored_first_pane, "ready").await;
    wait_for_screen_line(&fixture, restored.session.session_id, restored_second_pane, "ready")
        .await;

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
    assert_eq!(collect_pane_ids(&first_restored_tab.root).len(), 2);
    assert_eq!(collect_pane_ids(&second_restored_tab.root).len(), 1);
    let focused_tab = restored_topology.focused_tab.expect("focused tab should exist");
    let focused_tab = restored_topology
        .tabs
        .iter()
        .find(|tab| tab.tab_id == focused_tab)
        .expect("focused tab should exist");
    assert_eq!(focused_tab.title.as_deref(), Some("logs"));

    fixture.shutdown().await.expect("fixture should stop cleanly");
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_smoke_reports_incompatible_saved_session_manifest_via_daemon_api() {
    let (state, session_id) = daemon_state_with_incompatible_saved_session(
        "smoke-saved-incompat",
        SavedSessionManifest {
            format_version: CURRENT_SAVED_SESSION_FORMAT_VERSION,
            binary_version: CURRENT_BINARY_VERSION.to_string(),
            protocol_major: CURRENT_PROTOCOL_MAJOR,
            protocol_minor: CURRENT_PROTOCOL_MINOR + 1,
        },
    );
    let fixture =
        daemon_fixture_with_state("smoke-saved-incompat", state).expect("fixture should start");

    let listed =
        fixture.client.list_saved_sessions().await.expect("list_saved_sessions should succeed");
    let listed_session = listed
        .sessions
        .iter()
        .find(|session| session.session_id == session_id)
        .expect("saved session should be listed");
    let loaded =
        fixture.client.saved_session(session_id).await.expect("saved_session should succeed");
    let restore_error = fixture
        .client
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
    assert_eq!(restore_error.degraded_reason, Some(DegradedModeReason::SavedSessionIncompatible));

    fixture.shutdown().await.expect("fixture should stop cleanly");
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_smoke_prunes_saved_native_sessions_via_daemon_api() {
    let fixture = daemon_fixture_with_state(
        "bootstrap-native-prune-saved",
        isolated_daemon_state("bootstrap-native-prune-saved"),
    )
    .expect("fixture should start");
    let mut last_saved_session = None;

    for title in ["shell-a", "shell-b", "shell-c"] {
        let created = fixture
            .client
            .create_session(
                BackendKind::Native,
                CreateSessionSpec {
                    title: Some(title.to_string()),
                    launch: Some(cat_launch_spec()),
                },
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
        fixture
            .client
            .dispatch(created.session.session_id, MuxCommand::SaveSession)
            .await
            .expect("save session should succeed");
        last_saved_session = Some(created.session.session_id);
        thread::sleep(Duration::from_millis(5));
    }

    let pruned =
        fixture.client.prune_saved_sessions(1).await.expect("prune_saved_sessions should succeed");
    let listed =
        fixture.client.list_saved_sessions().await.expect("list_saved_sessions should succeed");

    assert_eq!(pruned.deleted_count, 2);
    assert_eq!(pruned.kept_count, 1);
    assert_eq!(listed.sessions.len(), 1);
    assert_eq!(
        listed.sessions[0].session_id,
        last_saved_session.expect("saved session id should exist")
    );

    fixture.shutdown().await.expect("fixture should stop cleanly");
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_smoke_overwrites_native_session_snapshot_on_resave() {
    let fixture = daemon_fixture("bootstrap-native-save-overwrite").expect("fixture should start");
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
    let tab_id = topology.focused_tab.expect("focused tab should exist");
    let pane_id = topology.tabs[0].focused_pane.expect("focused pane should exist");

    wait_for_screen_line(&fixture, created.session.session_id, pane_id, "ready").await;
    fixture
        .client
        .dispatch(created.session.session_id, MuxCommand::SaveSession)
        .await
        .expect("first save should succeed");
    let store = SqliteSessionStore::open_default().expect("default store should open");
    let first = store
        .load_native_session(created.session.session_id)
        .expect("first load should succeed")
        .expect("saved session should exist");

    fixture
        .client
        .dispatch(
            created.session.session_id,
            MuxCommand::RenameTab { tab_id, title: "shell-renamed".to_string() },
        )
        .await
        .expect("rename tab should succeed");
    fixture
        .client
        .dispatch(created.session.session_id, MuxCommand::SaveSession)
        .await
        .expect("second save should succeed");
    let second = store
        .load_native_session(created.session.session_id)
        .expect("second load should succeed")
        .expect("saved session should exist");

    assert_eq!(first.title.as_deref(), Some("shell"));
    assert_eq!(second.title.as_deref(), Some("shell-renamed"));
    assert_eq!(second.topology.tabs[0].title.as_deref(), Some("shell-renamed"));
    assert!(second.saved_at_ms >= first.saved_at_ms);

    fixture.shutdown().await.expect("fixture should stop cleanly");
}

#[cfg(any(unix, windows))]
#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_smoke_handles_rapid_native_tab_focus_churn() {
    let fixture = daemon_fixture("bootstrap-native-focus-churn").expect("fixture should start");
    let created = fixture
        .client
        .create_session(
            BackendKind::Native,
            CreateSessionSpec { title: Some("shell".to_string()), launch: Some(cat_launch_spec()) },
        )
        .await
        .expect("create_session should succeed");
    let mut subscription = fixture
        .client
        .open_subscription(created.session.session_id, SubscriptionSpec::SessionTopology)
        .await
        .expect("topology subscription should open");

    let initial = must_recv_subscription_event(&mut subscription).await;
    assert!(matches!(initial, SubscriptionEvent::TopologySnapshot(_)));

    for title in ["logs-a", "logs-b"] {
        fixture
            .client
            .dispatch(
                created.session.session_id,
                MuxCommand::NewTab(NewTabSpec { title: Some(title.to_string()) }),
            )
            .await
            .expect("new tab should succeed");
    }

    let initial_topology = wait_for_topology(
        &fixture,
        created.session.session_id,
        |snapshot| snapshot.tabs.len() == 3,
        "native tab churn setup",
    )
    .await;
    let tab_ids: Vec<TabId> = initial_topology.tabs.iter().map(|tab| tab.tab_id).collect();
    let focus_sequence = vec![
        tab_ids[1], tab_ids[2], tab_ids[0], tab_ids[2], tab_ids[1], tab_ids[0], tab_ids[2],
        tab_ids[1], tab_ids[0], tab_ids[2],
    ];
    let expected_final = *focus_sequence.last().expect("focus sequence should not be empty");

    for tab_id in &focus_sequence {
        fixture
            .client
            .dispatch(created.session.session_id, MuxCommand::FocusTab { tab_id: *tab_id })
            .await
            .expect("focus tab should succeed");
    }

    let final_topology = wait_for_topology(
        &fixture,
        created.session.session_id,
        |snapshot| snapshot.focused_tab == Some(expected_final),
        "native tab churn final focus",
    )
    .await;
    let mut saw_final_event = false;

    for _ in 0..48 {
        let next = tokio::time::timeout(host_timeout(), subscription.recv())
            .await
            .expect("native churn subscription should not hang")
            .expect("native churn subscription should stay healthy");
        let Some(SubscriptionEvent::TopologySnapshot(snapshot)) = next else {
            continue;
        };
        if snapshot.focused_tab == Some(expected_final) {
            saw_final_event = true;
            break;
        }
    }

    assert_eq!(final_topology.tabs.len(), 3);
    assert_eq!(final_topology.focused_tab, Some(expected_final));
    assert!(saw_final_event);

    subscription.close().await.expect("subscription should close cleanly");
    fixture.shutdown().await.expect("fixture should stop cleanly");
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_smoke_controls_native_pane_lifecycle_via_dispatch() {
    let fixture = daemon_fixture("bootstrap-native-pane-control").expect("fixture should start");
    let created = fixture
        .client
        .create_session(
            BackendKind::Native,
            CreateSessionSpec { title: Some("shell".to_string()), launch: Some(cat_launch_spec()) },
        )
        .await
        .expect("create_session should succeed");
    let initial = fixture
        .client
        .topology_snapshot(created.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let focused_tab = initial.focused_tab.expect("focused tab should exist");
    let tab = initial
        .tabs
        .iter()
        .find(|tab| tab.tab_id == focused_tab)
        .expect("focused tab should exist");
    let pane_id = tab.focused_pane.expect("focused pane should exist");
    wait_for_screen_line(&fixture, created.session.session_id, pane_id, "ready").await;
    let initial_screen = fixture
        .client
        .screen_snapshot(created.session.session_id, pane_id)
        .await
        .expect("screen_snapshot should succeed");

    let split = fixture
        .client
        .dispatch(
            created.session.session_id,
            MuxCommand::SplitPane(SplitPaneSpec { pane_id, direction: SplitDirection::Vertical }),
        )
        .await
        .expect("split pane should succeed");
    let after_split = fixture
        .client
        .topology_snapshot(created.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let split_tab = after_split
        .tabs
        .iter()
        .find(|tab| tab.tab_id == focused_tab)
        .expect("split tab should exist");
    let pane_ids = collect_pane_ids(&split_tab.root);
    let new_pane = pane_ids
        .iter()
        .copied()
        .find(|candidate| *candidate != pane_id)
        .expect("new pane should exist");

    wait_for_screen_line(&fixture, created.session.session_id, new_pane, "ready").await;
    let original_after_split = fixture
        .client
        .screen_snapshot(created.session.session_id, pane_id)
        .await
        .expect("screen_snapshot should succeed");
    let new_after_split = fixture
        .client
        .screen_snapshot(created.session.session_id, new_pane)
        .await
        .expect("screen_snapshot should succeed");
    let focus = fixture
        .client
        .dispatch(created.session.session_id, MuxCommand::FocusPane { pane_id })
        .await
        .expect("focus pane should succeed");
    let close = fixture
        .client
        .dispatch(created.session.session_id, MuxCommand::ClosePane { pane_id: new_pane })
        .await
        .expect("close pane should succeed");
    let after_close = fixture
        .client
        .topology_snapshot(created.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let restored_screen = fixture
        .client
        .screen_snapshot(created.session.session_id, pane_id)
        .await
        .expect("screen_snapshot should succeed");
    let close_last = fixture
        .client
        .dispatch(created.session.session_id, MuxCommand::ClosePane { pane_id })
        .await
        .expect_err("closing last pane should fail");

    assert!(split.changed);
    assert_eq!(split_tab.focused_pane, Some(new_pane));
    assert_eq!(original_after_split.rows, initial_screen.rows);
    assert_eq!(new_after_split.rows, initial_screen.rows);
    assert!(original_after_split.cols < initial_screen.cols);
    assert!(new_after_split.cols < initial_screen.cols);
    assert!(focus.changed);
    assert!(close.changed);
    assert_eq!(collect_pane_ids(&after_close.tabs[0].root), vec![pane_id]);
    assert_eq!(restored_screen.rows, initial_screen.rows);
    assert_eq!(restored_screen.cols, initial_screen.cols);
    assert_eq!(close_last.code, "backend_invalid_input");

    fixture.shutdown().await.expect("fixture should stop cleanly");
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_smoke_resizes_native_split_panes_through_layout_ratios() {
    let fixture = daemon_fixture("bootstrap-native-pane-resize").expect("fixture should start");
    let created = fixture
        .client
        .create_session(
            BackendKind::Native,
            CreateSessionSpec { title: Some("shell".to_string()), launch: Some(cat_launch_spec()) },
        )
        .await
        .expect("create_session should succeed");
    let initial = fixture
        .client
        .topology_snapshot(created.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let pane_id = initial.tabs[0].focused_pane.expect("focused pane should exist");

    wait_for_screen_line(&fixture, created.session.session_id, pane_id, "ready").await;
    fixture
        .client
        .dispatch(
            created.session.session_id,
            MuxCommand::SplitPane(SplitPaneSpec { pane_id, direction: SplitDirection::Vertical }),
        )
        .await
        .expect("split pane should succeed");
    let after_split = fixture
        .client
        .topology_snapshot(created.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let pane_ids = collect_pane_ids(&after_split.tabs[0].root);
    let new_pane = pane_ids
        .iter()
        .copied()
        .find(|candidate| *candidate != pane_id)
        .expect("new pane should exist");

    wait_for_screen_line(&fixture, created.session.session_id, new_pane, "ready").await;
    let original_before = fixture
        .client
        .screen_snapshot(created.session.session_id, pane_id)
        .await
        .expect("screen_snapshot should succeed");
    let target_before = fixture
        .client
        .screen_snapshot(created.session.session_id, new_pane)
        .await
        .expect("screen_snapshot should succeed");

    let resize_row = fixture
        .client
        .dispatch(
            created.session.session_id,
            MuxCommand::ResizePane(ResizePaneSpec {
                pane_id: new_pane,
                rows: target_before.rows.saturating_sub(4).max(4),
                cols: target_before.cols,
            }),
        )
        .await
        .expect_err("row resize should be rejected without horizontal split authority");
    let total_cols = original_before.cols.saturating_add(target_before.cols);
    let target_cols = target_before.cols.saturating_add(10).min(total_cols.saturating_sub(1));
    let resize = fixture
        .client
        .dispatch(
            created.session.session_id,
            MuxCommand::ResizePane(ResizePaneSpec {
                pane_id: new_pane,
                rows: target_before.rows,
                cols: target_cols,
            }),
        )
        .await
        .expect("col resize should succeed");
    let original_after = fixture
        .client
        .screen_snapshot(created.session.session_id, pane_id)
        .await
        .expect("screen_snapshot should succeed");
    let target_after = fixture
        .client
        .screen_snapshot(created.session.session_id, new_pane)
        .await
        .expect("screen_snapshot should succeed");

    assert_eq!(resize_row.code, "backend_unsupported");
    assert!(resize.changed);
    assert_eq!(target_after.rows, target_before.rows);
    assert_eq!(original_after.rows, original_before.rows);
    assert!(target_after.cols > target_before.cols);
    assert!(original_after.cols < original_before.cols);
    assert_eq!(target_after.cols + original_after.cols, target_before.cols + original_before.cols);

    fixture.shutdown().await.expect("fixture should stop cleanly");
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_smoke_overrides_native_layout_via_dispatch() {
    let fixture = daemon_fixture("bootstrap-native-layout-override").expect("fixture should start");
    let created = fixture
        .client
        .create_session(
            BackendKind::Native,
            CreateSessionSpec { title: Some("shell".to_string()), launch: Some(cat_launch_spec()) },
        )
        .await
        .expect("create_session should succeed");
    let initial = fixture
        .client
        .topology_snapshot(created.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let tab_id = initial.tabs[0].tab_id;
    let original_pane = initial.tabs[0].focused_pane.expect("focused pane should exist");

    wait_for_screen_line(&fixture, created.session.session_id, original_pane, "ready").await;
    fixture
        .client
        .dispatch(
            created.session.session_id,
            MuxCommand::SplitPane(SplitPaneSpec {
                pane_id: original_pane,
                direction: SplitDirection::Vertical,
            }),
        )
        .await
        .expect("split pane should succeed");
    let after_split = fixture
        .client
        .topology_snapshot(created.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let pane_ids = collect_pane_ids(&after_split.tabs[0].root);
    let new_pane = pane_ids
        .iter()
        .copied()
        .find(|candidate| *candidate != original_pane)
        .expect("new pane should exist");
    wait_for_screen_line(&fixture, created.session.session_id, new_pane, "ready").await;

    let original_before = fixture
        .client
        .screen_snapshot(created.session.session_id, original_pane)
        .await
        .expect("screen_snapshot should succeed");
    let new_before = fixture
        .client
        .screen_snapshot(created.session.session_id, new_pane)
        .await
        .expect("screen_snapshot should succeed");
    let override_layout = PaneTreeNode::Split(PaneSplit {
        direction: SplitDirection::Horizontal,
        first: Box::new(PaneTreeNode::Leaf { pane_id: original_pane }),
        second: Box::new(PaneTreeNode::Leaf { pane_id: new_pane }),
    });
    let override_result = fixture
        .client
        .dispatch(
            created.session.session_id,
            MuxCommand::OverrideLayout(OverrideLayoutSpec {
                tab_id,
                root: override_layout.clone(),
            }),
        )
        .await
        .expect("layout override should succeed");
    let invalid_override = fixture
        .client
        .dispatch(
            created.session.session_id,
            MuxCommand::OverrideLayout(OverrideLayoutSpec {
                tab_id,
                root: PaneTreeNode::Split(PaneSplit {
                    direction: SplitDirection::Horizontal,
                    first: Box::new(PaneTreeNode::Leaf { pane_id: original_pane }),
                    second: Box::new(PaneTreeNode::Leaf { pane_id: original_pane }),
                }),
            }),
        )
        .await
        .expect_err("duplicate pane ids should be rejected");
    let after_override = fixture
        .client
        .topology_snapshot(created.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let original_after = fixture
        .client
        .screen_snapshot(created.session.session_id, original_pane)
        .await
        .expect("screen_snapshot should succeed");
    let new_after = fixture
        .client
        .screen_snapshot(created.session.session_id, new_pane)
        .await
        .expect("screen_snapshot should succeed");

    assert!(override_result.changed);
    assert_eq!(after_override.tabs[0].root, override_layout);
    assert!(original_after.rows < original_before.rows);
    assert!(new_after.rows < new_before.rows);
    assert!(original_after.cols > original_before.cols);
    assert!(new_after.cols > new_before.cols);
    assert_eq!(invalid_override.code, "backend_invalid_input");

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
    let listed_after_rename =
        fixture.client.list_sessions().await.expect("list_sessions should succeed");
    let send_input = fixture
        .client
        .dispatch(
            imported.session.session_id,
            MuxCommand::SendInput(SendInputSpec {
                pane_id: focused_pane,
                data: submitted_input("hello from tmux dispatch"),
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
    assert_eq!(listed_after_rename.sessions[0].title.as_deref(), Some("workspace-renamed"));
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
async fn bootstrap_smoke_reads_inactive_tmux_tab_pane_snapshot() {
    let socket_name = unique_tmux_socket_name("bootstrap-tmux-inactive-pane");
    let session_name = unique_tmux_session_name("workspace");
    let _tmux =
        TmuxServerGuard::spawn(&socket_name, &session_name).expect("tmux test server should start");
    let fixture =
        daemon_fixture_with_state("bootstrap-tmux-inactive-pane", tmux_daemon_state(&socket_name))
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
    let inactive_pane = topology
        .tabs
        .iter()
        .find(|tab| tab.tab_id != topology.focused_tab.expect("focused tab should exist"))
        .and_then(|tab| collect_pane_ids(&tab.root).into_iter().next())
        .expect("inactive tmux tab pane should exist");
    let screen = fixture
        .client
        .screen_snapshot(imported.session.session_id, inactive_pane)
        .await
        .expect("screen_snapshot should succeed for inactive tmux pane");

    assert!(screen.surface.lines.iter().any(|line| line.text.contains("logs ready")));

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
    let before_resize = fixture
        .client
        .screen_snapshot(imported.session.session_id, new_pane)
        .await
        .expect("screen_snapshot should succeed");
    let resize = fixture
        .client
        .dispatch(
            imported.session.session_id,
            MuxCommand::ResizePane(ResizePaneSpec {
                pane_id: new_pane,
                rows: before_resize.rows.saturating_sub(4).max(4),
                cols: before_resize.cols.saturating_sub(6).max(10),
            }),
        )
        .await
        .expect("resize pane should succeed");
    let after_resize = fixture
        .client
        .screen_snapshot(imported.session.session_id, new_pane)
        .await
        .expect("screen_snapshot should succeed");

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
    let close = fixture
        .client
        .dispatch(imported.session.session_id, MuxCommand::ClosePane { pane_id: new_pane })
        .await
        .expect("close pane should succeed");
    let after_close = fixture
        .client
        .topology_snapshot(imported.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let focused_tab_after_close = after_close
        .tabs
        .iter()
        .find(|tab| tab.tab_id == focused_tab)
        .expect("focused tab snapshot should exist");
    let single_pane = after_close
        .tabs
        .iter()
        .find(|tab| tab.tab_id != focused_tab && collect_pane_ids(&tab.root).len() == 1)
        .and_then(|tab| collect_pane_ids(&tab.root).into_iter().next())
        .expect("single-pane secondary tab should exist");
    let close_last_error = fixture
        .client
        .dispatch(imported.session.session_id, MuxCommand::ClosePane { pane_id: single_pane })
        .await
        .expect_err("closing last pane in tab should be rejected");

    assert!(split.changed);
    assert_eq!(split_panes.len(), initial_panes.len() + 1);
    assert_eq!(split_tab.focused_pane, Some(new_pane));
    assert!(resize.changed);
    assert!(
        after_resize.rows != before_resize.rows || after_resize.cols != before_resize.cols,
        "resize should change at least one pane dimension"
    );
    assert!(focus.changed);
    assert_eq!(focused_tab_after_focus.focused_pane, Some(focused_pane));
    assert!(close.changed);
    assert_eq!(collect_pane_ids(&focused_tab_after_close.root).len(), initial_panes.len());
    assert_eq!(close_last_error.code, "backend_unsupported");
    assert_eq!(close_last_error.degraded_reason, Some(DegradedModeReason::UnsupportedByBackend));

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

    let initial = must_recv_subscription_event(&mut subscription).await;
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

    let updated = must_recv_subscription_event(&mut subscription).await;
    let mut updated = match updated {
        SubscriptionEvent::TopologySnapshot(snapshot) => snapshot,
        other => panic!("unexpected topology event: {other:?}"),
    };
    while updated.tabs.len() != 3 {
        let next = must_recv_subscription_event(&mut subscription).await;
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

    let initial = must_recv_subscription_event(&mut subscription).await;
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
                data: submitted_input("hello from tmux subscription"),
            }),
        )
        .await
        .expect("send input should succeed");

    let updated = loop {
        let next = must_recv_subscription_event(&mut subscription).await;
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
async fn bootstrap_smoke_preserves_tmux_fullscreen_viewports_for_vim_less_and_fzf() {
    let socket_name = unique_tmux_socket_name("bootstrap-tmux-fullscreen");
    let session_name = unique_tmux_session_name("workspace");
    let _tmux = TmuxServerGuard::spawn_with_shell(&socket_name, &session_name)
        .expect("tmux interactive test server should start");
    let fixture =
        daemon_fixture_with_state("bootstrap-tmux-fullscreen", tmux_daemon_state(&socket_name))
            .expect("fixture should start");

    let discovered = fixture
        .client
        .discover_sessions(BackendKind::Tmux)
        .await
        .expect("discover_sessions should succeed");
    let candidate = discovered
        .sessions
        .into_iter()
        .find(|session| session.title.as_deref() == Some(session_name.as_str()))
        .expect("importable tmux session should exist");
    let imported = fixture
        .client
        .import_session(candidate.route.clone(), candidate.title.clone())
        .await
        .expect("import_session should succeed");
    let topology = fixture
        .client
        .topology_snapshot(imported.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let focused_pane = topology.tabs[0].focused_pane.expect("focused pane should exist");

    wait_for_screen_line(&fixture, imported.session.session_id, focused_pane, "terminal-platform$")
        .await;

    let temp_name = format!(
        "terminal-platform-fullscreen-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default()
    );
    let viewport_file = std::env::temp_dir().join(format!("{temp_name}.txt"));
    let fzf_file = std::env::temp_dir().join(format!("{temp_name}-fzf.txt"));
    fs::write(
        &viewport_file,
        "tmux-vim-alpha\n\
tmux-vim-beta\n\
tmux-less-gamma\n\
tmux-less-delta\n",
    )
    .expect("viewport fixture file should write");
    fs::write(&fzf_file, "tmux-fzf-alpha\ntmux-fzf-beta\ntmux-fzf-gamma\n")
        .expect("fzf fixture file should write");

    fixture
        .client
        .dispatch(
            imported.session.session_id,
            MuxCommand::SendInput(SendInputSpec {
                pane_id: focused_pane,
                data: submitted_input(&format!("vim {}", viewport_file.display())),
            }),
        )
        .await
        .expect("vim should launch");
    wait_for_screen_line(&fixture, imported.session.session_id, focused_pane, "tmux-vim-alpha")
        .await;
    fixture
        .client
        .dispatch(
            imported.session.session_id,
            MuxCommand::SendInput(SendInputSpec {
                pane_id: focused_pane,
                data: submitted_input(":q!"),
            }),
        )
        .await
        .expect("vim should exit");
    wait_for_screen_line(&fixture, imported.session.session_id, focused_pane, "terminal-platform$")
        .await;

    fixture
        .client
        .dispatch(
            imported.session.session_id,
            MuxCommand::SendInput(SendInputSpec {
                pane_id: focused_pane,
                data: submitted_input(&format!("less {}", viewport_file.display())),
            }),
        )
        .await
        .expect("less should launch");
    wait_for_screen_line(&fixture, imported.session.session_id, focused_pane, "tmux-less-gamma")
        .await;
    fixture
        .client
        .dispatch(
            imported.session.session_id,
            MuxCommand::SendInput(SendInputSpec { pane_id: focused_pane, data: "q".to_string() }),
        )
        .await
        .expect("less should exit");
    wait_for_screen_line(&fixture, imported.session.session_id, focused_pane, "terminal-platform$")
        .await;

    fixture
        .client
        .dispatch(
            imported.session.session_id,
            MuxCommand::SendInput(SendInputSpec {
                pane_id: focused_pane,
                data: submitted_input(&format!("fzf < {}", fzf_file.display())),
            }),
        )
        .await
        .expect("fzf should launch");
    wait_for_screen_line(&fixture, imported.session.session_id, focused_pane, "tmux-fzf-beta")
        .await;
    fixture
        .client
        .dispatch(
            imported.session.session_id,
            MuxCommand::SendInput(SendInputSpec {
                pane_id: focused_pane,
                data: submitted_input("beta"),
            }),
        )
        .await
        .expect("fzf should accept selection");
    wait_for_screen_line(&fixture, imported.session.session_id, focused_pane, "tmux-fzf-beta")
        .await;
    wait_for_screen_line(&fixture, imported.session.session_id, focused_pane, "terminal-platform$")
        .await;

    let _ = fs::remove_file(viewport_file);
    let _ = fs::remove_file(fzf_file);

    fixture.shutdown().await.expect("fixture should stop cleanly");
}

#[cfg(any(unix, windows))]
#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_smoke_discovers_zellij_session_and_handles_import_surface() {
    let _zellij_lock = ZellijTestLock::acquire().expect("zellij test lock should acquire");
    let attempts = if cfg!(windows) { 1 } else { 3 };
    let mut last_error = None;

    for attempt in 0..attempts {
        let run = tokio::spawn(async move {
            timeout(zellij_attempt_timeout(), async move {
                let session_name = unique_zellij_session_name("workspace");
                let _zellij =
                    ZellijSessionGuard::spawn(&session_name).expect("zellij session should start");
                let fixture =
                    daemon_fixture("bootstrap-zellij-discover").expect("fixture should start");
                let capabilities = fixture
                    .client
                    .backend_capabilities(BackendKind::Zellij)
                    .await
                    .expect("zellij capabilities should succeed");

                let candidate =
                    wait_for_discovered_zellij_session(&fixture.client, &session_name).await;
                assert_eq!(candidate.route.backend, BackendKind::Zellij);

                if !capabilities.capabilities.rendered_viewport_snapshot {
                    let error = tokio::time::timeout(
                        host_timeout(),
                        fixture
                            .client
                            .import_session(candidate.route.clone(), candidate.title.clone()),
                    )
                    .await
                    .expect("import_session should not hang")
                    .expect_err("legacy local zellij surface should reject imported attach");
                    let listed =
                        fixture.client.list_sessions().await.expect("list_sessions should succeed");

                    assert_eq!(error.code, "backend_unsupported");
                    assert!(error.message.contains("zellij 0.43.1"));
                    assert_eq!(error.degraded_reason, Some(DegradedModeReason::MissingCapability));
                    assert!(listed.sessions.is_empty());
                } else {
                    let imported = tokio::time::timeout(
                        zellij_operation_timeout(),
                        fixture
                            .client
                            .import_session(candidate.route.clone(), candidate.title.clone()),
                    )
                    .await
                    .expect("import_session should not hang")
                    .expect("rich zellij surface should import successfully");
                    let listed =
                        fixture.client.list_sessions().await.expect("list_sessions should succeed");
                    let topology = tokio::time::timeout(
                        host_timeout(),
                        fixture.client.topology_snapshot(imported.session.session_id),
                    )
                    .await
                    .expect("topology_snapshot should not hang")
                    .expect("topology_snapshot should succeed");
                    let focused_tab = topology
                        .tabs
                        .iter()
                        .find(|tab| Some(tab.tab_id) == topology.focused_tab)
                        .or_else(|| topology.tabs.first())
                        .expect("zellij topology should have tabs");
                    let focused_pane = focused_tab
                        .focused_pane
                        .or_else(|| collect_pane_ids(&focused_tab.root).first().copied())
                        .expect("focused zellij pane should exist");
                    let screen = tokio::time::timeout(
                        host_timeout(),
                        fixture.client.screen_snapshot(imported.session.session_id, focused_pane),
                    )
                    .await
                    .expect("screen_snapshot should not hang")
                    .expect("screen_snapshot should succeed");
                    let delta = tokio::time::timeout(
                        host_timeout(),
                        fixture.client.screen_delta(
                            imported.session.session_id,
                            focused_pane,
                            screen.sequence,
                        ),
                    )
                    .await
                    .expect("screen_delta should not hang")
                    .expect("screen_delta should succeed");
                    let mut topology_subscription = fixture
                        .client
                        .open_subscription(
                            imported.session.session_id,
                            SubscriptionSpec::SessionTopology,
                        )
                        .await
                        .expect("zellij topology subscription should open");
                    let mut pane_subscription = fixture
                        .client
                        .open_subscription(
                            imported.session.session_id,
                            SubscriptionSpec::PaneSurface { pane_id: focused_pane },
                        )
                        .await
                        .expect("zellij pane subscription should open");
                    let initial_topology =
                        tokio::time::timeout(host_timeout(), topology_subscription.recv())
                            .await
                            .expect("zellij topology subscription should not hang")
                            .expect("zellij topology subscription should stay healthy")
                            .expect("zellij topology subscription should emit initial event");
                    let initial_pane =
                        tokio::time::timeout(host_timeout(), pane_subscription.recv())
                            .await
                            .expect("zellij pane subscription should not hang")
                            .expect("zellij pane subscription should stay healthy")
                            .expect("zellij pane subscription should emit initial event");

                    assert_eq!(imported.session.route.backend, BackendKind::Zellij);
                    assert!(
                        listed
                            .sessions
                            .iter()
                            .any(|session| session.session_id == imported.session.session_id)
                    );
                    assert_eq!(topology.backend_kind, BackendKind::Zellij);
                    assert!(!topology.tabs.is_empty());
                    assert_eq!(screen.pane_id, focused_pane);
                    assert_eq!(screen.source, ProjectionSource::ZellijDumpSnapshot);
                    assert_zellij_delta_compatible_with_snapshot(&screen, &delta);
                    match initial_topology {
                        SubscriptionEvent::TopologySnapshot(snapshot) => {
                            assert_eq!(snapshot.session_id, imported.session.session_id);
                            assert_eq!(snapshot.backend_kind, BackendKind::Zellij);
                        }
                        other => panic!("unexpected initial zellij topology event: {other:?}"),
                    }
                    match initial_pane {
                        SubscriptionEvent::ScreenDelta(delta) => {
                            assert_eq!(delta.pane_id, focused_pane);
                            assert_eq!(delta.source, ProjectionSource::ZellijDumpSnapshot);
                            assert!(delta.full_replace.is_some());
                        }
                        other => panic!("unexpected initial zellij pane event: {other:?}"),
                    }

                    let initial_tab_count = topology.tabs.len();
                    let initial_focused_tab =
                        topology.focused_tab.expect("focused zellij tab should exist");

                    let created = tokio::time::timeout(
                        zellij_operation_timeout(),
                        fixture.client.dispatch(
                            imported.session.session_id,
                            MuxCommand::NewTab(NewTabSpec { title: Some("logs-rich".to_string()) }),
                        ),
                    )
                    .await
                    .expect("zellij new_tab should not hang")
                    .expect("zellij new_tab should succeed");
                    let after_create = wait_for_topology(
                        &fixture,
                        imported.session.session_id,
                        |snapshot| {
                            snapshot.tabs.len() == initial_tab_count + 1
                                && snapshot
                                    .tabs
                                    .iter()
                                    .any(|tab| tab.title.as_deref() == Some("logs-rich"))
                        },
                        "zellij rich new tab topology",
                    )
                    .await;
                    let rich_tab_id = after_create
                        .tabs
                        .iter()
                        .find(|tab| tab.title.as_deref() == Some("logs-rich"))
                        .map(|tab| tab.tab_id)
                        .expect("created rich zellij tab should exist");

                    let renamed = tokio::time::timeout(
                        zellij_operation_timeout(),
                        fixture.client.dispatch(
                            imported.session.session_id,
                            MuxCommand::RenameTab {
                                tab_id: rich_tab_id,
                                title: "logs-rich-renamed".to_string(),
                            },
                        ),
                    )
                    .await
                    .expect("zellij rename_tab should not hang")
                    .expect("zellij rename_tab should succeed");
                    let after_rename = wait_for_topology(
                        &fixture,
                        imported.session.session_id,
                        |snapshot| {
                            snapshot.tabs.iter().any(|tab| {
                                tab.tab_id == rich_tab_id
                                    && tab.title.as_deref() == Some("logs-rich-renamed")
                            })
                        },
                        "zellij rich renamed tab topology",
                    )
                    .await;

                    let focused = tokio::time::timeout(
                        zellij_operation_timeout(),
                        fixture.client.dispatch(
                            imported.session.session_id,
                            MuxCommand::FocusTab { tab_id: initial_focused_tab },
                        ),
                    )
                    .await
                    .expect("zellij focus_tab should not hang")
                    .expect("zellij focus_tab should succeed");
                    let after_focus = wait_for_topology(
                        &fixture,
                        imported.session.session_id,
                        |snapshot| snapshot.focused_tab == Some(initial_focused_tab),
                        "zellij rich focus tab topology",
                    )
                    .await;

                    let closed = tokio::time::timeout(
                        zellij_operation_timeout(),
                        fixture.client.dispatch(
                            imported.session.session_id,
                            MuxCommand::CloseTab { tab_id: rich_tab_id },
                        ),
                    )
                    .await
                    .expect("zellij close_tab should not hang")
                    .expect("zellij close_tab should succeed");
                    let after_close = wait_for_topology(
                        &fixture,
                        imported.session.session_id,
                        |snapshot| {
                            snapshot.tabs.len() == initial_tab_count
                                && snapshot.tabs.iter().all(|tab| tab.tab_id != rich_tab_id)
                        },
                        "zellij rich close tab topology",
                    )
                    .await;

                    assert!(created.changed);
                    assert_eq!(after_create.tabs.len(), initial_tab_count + 1);
                    assert!(renamed.changed);
                    assert!(after_rename.tabs.iter().any(|tab| {
                        tab.tab_id == rich_tab_id
                            && tab.title.as_deref() == Some("logs-rich-renamed")
                    }));
                    assert!(focused.changed);
                    assert_eq!(after_focus.focused_tab, Some(initial_focused_tab));
                    assert!(closed.changed);
                    assert_eq!(after_close.tabs.len(), initial_tab_count);

                    topology_subscription
                        .close()
                        .await
                        .expect("topology subscription should close cleanly");
                    pane_subscription
                        .close()
                        .await
                        .expect("pane subscription should close cleanly");
                }

                fixture.shutdown().await.expect("fixture should stop cleanly");
            })
            .await
            .expect("bootstrap zellij smoke attempt should complete within timeout");
        });

        match run.await {
            Ok(()) => return,
            Err(error) => {
                last_error = Some(format!("attempt {} failed: {error}", attempt + 1));
                sleep(Duration::from_millis(250)).await;
            }
        }
    }

    panic!(
        "bootstrap zellij import smoke failed after {attempts} attempts: {}",
        last_error.unwrap_or_else(|| "unknown failure".to_string())
    );
}

#[cfg(any(unix, windows))]
#[ignore = "extended zellij stress coverage exceeds the portable CI latency budget"]
#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_smoke_handles_rapid_zellij_tab_focus_churn() {
    let session_name = unique_zellij_session_name("focus");
    let _zellij = ZellijSessionGuard::spawn(&session_name).expect("zellij session should start");
    let fixture = daemon_fixture("bootstrap-zellij-focus-churn").expect("fixture should start");
    let capabilities = fixture
        .client
        .backend_capabilities(BackendKind::Zellij)
        .await
        .expect("zellij capabilities should succeed");

    if !capabilities.capabilities.rendered_viewport_snapshot {
        fixture.shutdown().await.expect("fixture should stop cleanly");
        return;
    }

    let candidate = wait_for_discovered_zellij_session(&fixture.client, &session_name).await;
    let imported = tokio::time::timeout(
        zellij_operation_timeout(),
        fixture.client.import_session(candidate.route, candidate.title),
    )
    .await
    .expect("import_session should not hang")
    .expect("zellij import should succeed");
    let mut subscription = fixture
        .client
        .open_subscription(imported.session.session_id, SubscriptionSpec::SessionTopology)
        .await
        .expect("topology subscription should open");

    let initial = must_recv_subscription_event(&mut subscription).await;
    assert!(matches!(initial, SubscriptionEvent::TopologySnapshot(_)));

    for title in ["focus-a", "focus-b"] {
        tokio::time::timeout(
            zellij_operation_timeout(),
            fixture.client.dispatch(
                imported.session.session_id,
                MuxCommand::NewTab(NewTabSpec { title: Some(title.to_string()) }),
            ),
        )
        .await
        .expect("zellij new_tab should not hang")
        .expect("zellij new_tab should succeed");
    }

    let initial_topology = wait_for_topology(
        &fixture,
        imported.session.session_id,
        |snapshot| {
            snapshot.tabs.len() >= 3
                && snapshot.tabs.iter().any(|tab| tab.title.as_deref() == Some("focus-a"))
                && snapshot.tabs.iter().any(|tab| tab.title.as_deref() == Some("focus-b"))
        },
        "zellij focus churn setup",
    )
    .await;
    let tab_ids: Vec<TabId> = initial_topology.tabs.iter().map(|tab| tab.tab_id).collect();
    let focus_sequence = vec![tab_ids[1], tab_ids[2], tab_ids[0], tab_ids[2]];
    let expected_final = *focus_sequence.last().expect("focus sequence should not be empty");

    for tab_id in &focus_sequence {
        tokio::time::timeout(
            zellij_operation_timeout(),
            fixture
                .client
                .dispatch(imported.session.session_id, MuxCommand::FocusTab { tab_id: *tab_id }),
        )
        .await
        .expect("zellij focus_tab should not hang")
        .expect("zellij focus_tab should succeed");
    }

    let final_topology = wait_for_topology(
        &fixture,
        imported.session.session_id,
        |snapshot| snapshot.focused_tab == Some(expected_final),
        "zellij focus churn final focus",
    )
    .await;
    let mut saw_final_event = false;

    for _ in 0..48 {
        let next = tokio::time::timeout(host_timeout(), subscription.recv())
            .await
            .expect("zellij churn subscription should not hang")
            .expect("zellij churn subscription should stay healthy");
        let Some(SubscriptionEvent::TopologySnapshot(snapshot)) = next else {
            continue;
        };
        if snapshot.focused_tab == Some(expected_final) {
            saw_final_event = true;
            break;
        }
    }

    let focused_tab = final_topology
        .tabs
        .iter()
        .find(|tab| tab.tab_id == expected_final)
        .expect("final focused tab should exist");
    let focused_pane = focused_tab
        .focused_pane
        .or_else(|| collect_pane_ids(&focused_tab.root).first().copied())
        .expect("focused pane should exist");
    let final_screen = fixture
        .client
        .screen_snapshot(imported.session.session_id, focused_pane)
        .await
        .expect("screen_snapshot should succeed");

    assert!(saw_final_event);
    assert_eq!(final_topology.focused_tab, Some(expected_final));
    assert_eq!(final_screen.pane_id, focused_pane);
    assert_eq!(final_screen.source, ProjectionSource::ZellijDumpSnapshot);

    subscription.close().await.expect("subscription should close cleanly");
    fixture.shutdown().await.expect("fixture should stop cleanly");
}

#[cfg(any(unix, windows))]
fn cat_launch_spec() -> ShellLaunchSpec {
    echo_shell_launch_spec()
}

#[cfg(unix)]
fn daemon_state_with_incompatible_saved_session(
    label: &str,
    manifest: SavedSessionManifest,
) -> (TerminalDaemonState, SessionId) {
    let store = SqliteSessionStore::open(unique_sqlite_path(label))
        .expect("isolated sqlite session store should open");
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

    (TerminalDaemonState::with_default_persistence(store), session_id)
}

#[cfg(any(unix, windows))]
async fn wait_for_screen_line(
    fixture: &terminal_testing::DaemonFixture,
    session_id: terminal_domain::SessionId,
    pane_id: terminal_domain::PaneId,
    needle: &str,
) {
    let mut last_lines = Vec::new();
    for _ in 0..120 {
        let screen = fixture
            .client
            .screen_snapshot(session_id, pane_id)
            .await
            .expect("screen_snapshot should succeed");
        if screen.surface.lines.iter().any(|line| line.text.contains(needle)) {
            return;
        }
        last_lines = screen.surface.lines.iter().map(|line| line.text.clone()).take(12).collect();
        sleep(Duration::from_millis(50)).await;
    }

    panic!("screen never contained expected text: {needle}; last lines: {last_lines:?}");
}

#[cfg(any(unix, windows))]
async fn wait_for_topology(
    fixture: &terminal_testing::DaemonFixture,
    session_id: terminal_domain::SessionId,
    predicate: impl Fn(&TopologySnapshot) -> bool,
    label: &str,
) -> TopologySnapshot {
    let attempts = if label.contains("zellij") { zellij_topology_wait_attempts() } else { 120 };
    let mut last_snapshot = None;
    for _ in 0..attempts {
        let snapshot =
            tokio::time::timeout(host_timeout(), fixture.client.topology_snapshot(session_id))
                .await
                .expect("topology_snapshot should not hang")
                .expect("topology_snapshot should succeed");
        if predicate(&snapshot) {
            return snapshot;
        }
        last_snapshot = Some(snapshot);
        sleep(Duration::from_millis(50)).await;
    }

    panic!("topology never reached expected state: {label}; last snapshot: {last_snapshot:?}");
}

#[cfg(any(unix, windows))]
async fn recv_subscription_event(
    subscription: &mut terminal_daemon_client::LocalSocketSubscription,
) -> Option<SubscriptionEvent> {
    tokio::time::timeout(host_timeout(), subscription.recv())
        .await
        .expect("subscription recv should not hang")
        .expect("subscription recv should succeed")
}

#[cfg(any(unix, windows))]
fn zellij_operation_timeout() -> Duration {
    if cfg!(windows) { Duration::from_secs(60) } else { Duration::from_secs(90) }
}

#[cfg(any(unix, windows))]
fn zellij_attempt_timeout() -> Duration {
    if cfg!(windows) { Duration::from_secs(120) } else { Duration::from_secs(90) }
}

#[cfg(any(unix, windows))]
fn host_timeout() -> Duration {
    if cfg!(windows) { Duration::from_secs(45) } else { Duration::from_secs(10) }
}

#[cfg(any(unix, windows))]
fn zellij_topology_wait_attempts() -> usize {
    if cfg!(windows) { 80 } else { 120 }
}

#[cfg(any(unix, windows))]
fn assert_zellij_delta_compatible_with_snapshot(snapshot: &ScreenSnapshot, delta: &ScreenDelta) {
    assert_eq!(delta.from_sequence, snapshot.sequence);
    assert!(
        delta.to_sequence >= snapshot.sequence,
        "zellij delta must not rewind sequence numbers"
    );
    if delta.to_sequence == snapshot.sequence {
        assert!(delta.patch.is_none());
        assert!(delta.full_replace.is_none());
    } else {
        assert!(delta.patch.is_none());
        assert!(delta.full_replace.is_some());
    }
}

#[cfg(any(unix, windows))]
async fn wait_for_discovered_zellij_session(
    client: &terminal_daemon_client::LocalSocketDaemonClient,
    session_name: &str,
) -> terminal_backend_api::DiscoveredSession {
    let started = Instant::now();
    while started.elapsed() < zellij_discovery_timeout() {
        let discovered = match tokio::time::timeout(
            host_timeout(),
            client.discover_sessions(BackendKind::Zellij),
        )
        .await
        {
            Ok(Ok(discovered)) => discovered,
            Ok(Err(_)) | Err(_) => break,
        };
        if let Some(candidate) = discovered
            .sessions
            .into_iter()
            .find(|session| session.title.as_deref() == Some(session_name))
        {
            return candidate;
        }
        sleep(Duration::from_millis(100)).await;
    }

    fallback_zellij_candidate(session_name)
}

#[cfg(any(unix, windows))]
fn zellij_discovery_timeout() -> Duration {
    if cfg!(windows) { Duration::from_secs(30) } else { Duration::from_secs(20) }
}

#[cfg(any(unix, windows))]
fn fallback_zellij_candidate(session_name: &str) -> terminal_backend_api::DiscoveredSession {
    terminal_backend_api::DiscoveredSession {
        route: terminal_domain::SessionRoute {
            backend: BackendKind::Zellij,
            authority: terminal_domain::RouteAuthority::ImportedForeign,
            external: Some(terminal_domain::ExternalSessionRef {
                namespace: "zellij_session".to_string(),
                value: format!("session={session_name}"),
            }),
        },
        title: Some(session_name.to_string()),
    }
}

#[cfg(any(unix, windows))]
fn submitted_input(text: &str) -> String {
    if cfg!(windows) { format!("echo {text}\r") } else { format!("{text}\r") }
}

#[cfg(any(unix, windows))]
async fn must_recv_subscription_event(
    subscription: &mut terminal_daemon_client::LocalSocketSubscription,
) -> SubscriptionEvent {
    recv_subscription_event(subscription).await.expect("subscription should emit an event")
}

#[cfg(any(unix, windows))]
fn collect_pane_ids(root: &PaneTreeNode) -> Vec<PaneId> {
    let mut pane_ids = Vec::new();
    collect_pane_ids_inner(root, &mut pane_ids);
    pane_ids
}

#[cfg(any(unix, windows))]
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
        Self::spawn_with_commands(
            socket_name,
            session_name,
            "printf 'hello from tmux\\n'; exec cat",
            "printf 'logs ready\\n'; exec cat",
        )
    }

    fn spawn_with_shell(socket_name: &str, session_name: &str) -> Result<Self, String> {
        Self::spawn_with_commands(
            socket_name,
            session_name,
            "printf 'hello from tmux\\n'; exec env PS1='terminal-platform$ ' sh -i",
            "printf 'logs ready\\n'; exec env PS1='terminal-platform$ ' sh -i",
        )
    }

    fn spawn_with_commands(
        socket_name: &str,
        session_name: &str,
        main_command: &str,
        secondary_command: &str,
    ) -> Result<Self, String> {
        run_tmux(
            socket_name,
            &["new-session", "-d", "-s", session_name, "sh", "-lc", main_command],
        )?;
        run_tmux(
            socket_name,
            &["new-window", "-d", "-t", session_name, "-n", "logs", "sh", "-lc", secondary_command],
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
