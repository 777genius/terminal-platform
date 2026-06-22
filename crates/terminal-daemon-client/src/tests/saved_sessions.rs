use super::{prelude::*, support::*};

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn lists_and_loads_saved_native_sessions() {
    let address = unique_address("daemon-client-saved");
    let server =
        spawn_local_socket_server(isolated_daemon(), address.clone()).expect("server should bind");
    let client = LocalSocketDaemonClient::new(address);
    let created = client
        .create_session(
            BackendKind::Native,
            CreateSessionSpec { title: Some("shell".to_string()), launch: Some(cat_launch_spec()) },
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
    assert!(!saved_summary.restore_semantics.preserves_process_state);
    let summary_v2 = saved_summary
        .restore_semantics_v2
        .as_ref()
        .expect("saved summary should expose v2 restore semantics");
    assert_eq!(
        saved_summary.restore_semantics.replays_saved_screen_buffers,
        summary_v2.replays_saved_screen_buffers
    );
    assert!(matches!(
        summary_v2.restore_guarantee_level,
        RestoreGuaranteeLevel::RichHistory
            | RestoreGuaranteeLevel::BasicHistory
            | RestoreGuaranteeLevel::VisualRestoreOnly
    ));
    assert!(matches!(
        summary_v2.history_replay_state,
        HistoryReplayState::ReplayedFromJournal | HistoryReplayState::HydratedFromSnapshot
    ));
    assert_eq!(loaded.session.session_id, created.session.session_id);
    assert_eq!(loaded.session.title.as_deref(), Some("shell"));
    assert_eq!(loaded.session.topology.tabs.len(), 1);
    assert_eq!(loaded.session.screens.len(), 1);
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
    assert!(matches!(
        loaded_v2.restore_guarantee_level,
        RestoreGuaranteeLevel::RichHistory
            | RestoreGuaranteeLevel::BasicHistory
            | RestoreGuaranteeLevel::VisualRestoreOnly
    ));
    assert!(matches!(
        loaded_v2.history_replay_state,
        HistoryReplayState::ReplayedFromJournal | HistoryReplayState::HydratedFromSnapshot
    ));

    server.shutdown().await.expect("server shutdown should succeed");
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn deletes_saved_native_sessions() {
    let address = unique_address("daemon-client-saved-delete");
    let server =
        spawn_local_socket_server(isolated_daemon(), address.clone()).expect("server should bind");
    let client = LocalSocketDaemonClient::new(address);
    let created = client
        .create_session(
            BackendKind::Native,
            CreateSessionSpec { title: Some("shell".to_string()), launch: Some(cat_launch_spec()) },
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
    let listed = client.list_saved_sessions().await.expect("list_saved_sessions should succeed");
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
    let server =
        spawn_local_socket_server(isolated_daemon(), address.clone()).expect("server should bind");
    let client = LocalSocketDaemonClient::new(address);
    let created = client
        .create_session(
            BackendKind::Native,
            CreateSessionSpec { title: Some("shell".to_string()), launch: Some(cat_launch_spec()) },
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
    assert!(!restored.restore_semantics.preserves_process_state);
    let restored_v2 = restored
        .restore_semantics_v2
        .as_ref()
        .expect("restored saved session should expose v2 restore semantics");
    assert_eq!(
        restored.restore_semantics.replays_saved_screen_buffers,
        restored_v2.replays_saved_screen_buffers
    );
    assert!(matches!(
        restored_v2.restore_guarantee_level,
        RestoreGuaranteeLevel::RichHistory
            | RestoreGuaranteeLevel::BasicHistory
            | RestoreGuaranteeLevel::VisualRestoreOnly
    ));
    assert!(matches!(
        restored_v2.history_replay_state,
        HistoryReplayState::ReplayedFromJournal | HistoryReplayState::HydratedFromSnapshot
    ));
    assert_eq!(restored_v2.source_session_id, created.session.session_id);
    assert_eq!(restored_v2.restored_session_id, Some(restored.session.session_id));
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
    let server = spawn_local_socket_server(daemon, address.clone()).expect("server should bind");
    let client = LocalSocketDaemonClient::new(address);

    let listed = client.list_saved_sessions().await.expect("list_saved_sessions should succeed");
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
    assert_eq!(restore_error.degraded_reason, Some(DegradedModeReason::SavedSessionIncompatible));

    server.shutdown().await.expect("server shutdown should succeed");
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn list_saved_sessions_skips_corrupted_rows() {
    let (daemon, valid_session_id, corrupt_session_id) =
        isolated_daemon_with_valid_and_corrupted_saved_rows("daemon-client-saved-corrupt");
    let address = unique_address("daemon-client-saved-corrupt");
    let server = spawn_local_socket_server(daemon, address.clone()).expect("server should bind");
    let client = LocalSocketDaemonClient::new(address);

    let listed = client.list_saved_sessions().await.expect("list_saved_sessions should succeed");
    let loaded =
        client.saved_session(valid_session_id).await.expect("valid saved_session should succeed");
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
    let server =
        spawn_local_socket_server(isolated_daemon(), address.clone()).expect("server should bind");
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

    let pruned = client.prune_saved_sessions(1).await.expect("prune_saved_sessions should succeed");
    let listed = client.list_saved_sessions().await.expect("list_saved_sessions should succeed");

    assert_eq!(pruned.deleted_count, 2);
    assert_eq!(pruned.kept_count, 1);
    assert_eq!(listed.sessions.len(), 1);
    assert_eq!(
        listed.sessions[0].session_id,
        last_saved_session.expect("saved session id should exist")
    );

    server.shutdown().await.expect("server shutdown should succeed");
}
