use super::super::{prelude::*, support::*};

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
