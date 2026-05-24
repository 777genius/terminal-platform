use super::super::{prelude::*, support::*};

#[cfg(any(unix, windows))]
#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_smoke_save_v2_failure_does_not_publish_visible_saved_session() {
    let store_path = unique_sqlite_path("bootstrap-native-save-v2-failpoint");
    let mut config = TerminalPersistenceV2Config::test();
    config.failpoints.saved_session_v2_snapshot_before_import = true;
    let store = SqliteSessionStore::open_with_v2_config(&store_path, config)
        .expect("isolated sqlite store should open with failpoint config");
    let fixture = daemon_fixture_with_daemon(
        "bootstrap-native-save-v2-failpoint",
        TerminalDaemon::with_persistence(store.clone()),
    )
    .expect("fixture should start");
    let created = fixture
        .client
        .create_session(
            BackendKind::Native,
            CreateSessionSpec {
                title: Some("save-failpoint-shell".to_string()),
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
    let save_error = fixture
        .client
        .dispatch(created.session.session_id, MuxCommand::SaveSession)
        .await
        .expect_err("save should fail before legacy publication when v2 evidence fails");
    let listed = fixture
        .client
        .list_saved_sessions()
        .await
        .expect("list_saved_sessions should still succeed after failed save");
    let lookup_error = fixture
        .client
        .saved_session(created.session.session_id)
        .await
        .expect_err("failed v2 save must not publish a visible saved session");
    let legacy_saved = store
        .load_native_session(created.session.session_id)
        .expect("direct legacy lookup should succeed");

    assert_eq!(save_error.code, "backend_internal");
    assert!(save_error.message.contains("saved_session_v2_snapshot_before_import"));
    assert!(
        !listed.sessions.iter().any(|session| session.session_id == created.session.session_id)
    );
    assert_eq!(lookup_error.code, "backend_not_found");
    assert!(legacy_saved.is_none());

    fixture.shutdown().await.expect("fixture should stop cleanly");
}

#[cfg(any(unix, windows))]
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

#[cfg(any(unix, windows))]
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
    assert!(!saved_summary.restore_semantics.preserves_process_state);
    let saved_summary_v2 = saved_summary
        .restore_semantics_v2
        .as_ref()
        .expect("saved session summary should expose v2 restore semantics");
    assert_eq!(
        saved_summary.restore_semantics.replays_saved_screen_buffers,
        saved_summary_v2.replays_saved_screen_buffers
    );
    assert_eq!(saved_summary_v2.source_session_id, created.session.session_id);
    assert_eq!(saved_summary_v2.restored_session_id, None);
    assert!(saved_summary_v2.restores_topology);
    assert!(saved_summary_v2.restores_focus_state);
    assert!(saved_summary_v2.restores_tab_titles);
    assert!(saved_summary_v2.uses_saved_launch_spec);
    assert!(!saved_summary_v2.preserves_process_state);
    assert_ne!(
        saved_summary_v2.history_replay_state,
        terminal_protocol::HistoryReplayState::NotAvailable
    );
    assert!(!saved_summary_v2.evidence_refs.is_empty());
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
    assert!(!loaded.session.restore_semantics.preserves_process_state);
    let loaded_v2 = loaded
        .session
        .restore_semantics_v2
        .as_ref()
        .expect("loaded saved session should expose v2 restore semantics");
    assert_eq!(
        loaded.session.restore_semantics.replays_saved_screen_buffers,
        loaded_v2.replays_saved_screen_buffers
    );
    assert_eq!(loaded_v2.source_session_id, created.session.session_id);
    assert_eq!(loaded_v2.restored_session_id, None);
    assert!(loaded_v2.restores_topology);
    assert!(loaded_v2.restores_focus_state);
    assert!(loaded_v2.restores_tab_titles);
    assert!(loaded_v2.uses_saved_launch_spec);
    assert_ne!(loaded_v2.history_replay_state, terminal_protocol::HistoryReplayState::NotAvailable);

    fixture.shutdown().await.expect("fixture should stop cleanly");
}

#[cfg(any(unix, windows))]
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
