use super::super::{prelude::*, support::*};

#[cfg(any(unix, windows))]
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
    assert!(!restored.restore_semantics.preserves_process_state);
    let restored_v2 = restored
        .restore_semantics_v2
        .as_ref()
        .expect("restore response should expose v2 restore semantics");
    assert_eq!(
        restored.restore_semantics.replays_saved_screen_buffers,
        restored_v2.replays_saved_screen_buffers
    );
    assert_eq!(restored_v2.source_session_id, created.session.session_id);
    assert_eq!(restored_v2.restored_session_id, Some(restored.session.session_id));
    assert!(restored_v2.restores_topology);
    assert!(restored_v2.restores_focus_state);
    assert!(restored_v2.restores_tab_titles);
    assert!(restored_v2.uses_saved_launch_spec);
    assert!(!restored_v2.preserves_process_state);
    assert_ne!(
        restored_v2.history_replay_state,
        terminal_protocol::HistoryReplayState::NotAvailable
    );
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

#[cfg(any(unix, windows))]
#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_smoke_reports_incompatible_saved_session_manifest_via_daemon_api() {
    let (state, session_id) = daemon_with_incompatible_saved_session(
        "smoke-saved-incompat",
        SavedSessionManifest {
            format_version: CURRENT_SAVED_SESSION_FORMAT_VERSION,
            binary_version: CURRENT_BINARY_VERSION.to_string(),
            protocol_major: CURRENT_PROTOCOL_MAJOR,
            protocol_minor: CURRENT_PROTOCOL_MINOR + 1,
        },
    );
    let fixture =
        daemon_fixture_with_daemon("smoke-saved-incompat", state).expect("fixture should start");

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
