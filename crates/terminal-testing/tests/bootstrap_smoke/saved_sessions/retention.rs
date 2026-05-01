use super::super::{prelude::*, support::*};

#[cfg(any(unix, windows))]
#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_smoke_prunes_saved_native_sessions_via_daemon_api() {
    let fixture = daemon_fixture_with_daemon(
        "bootstrap-native-prune-saved",
        isolated_daemon("bootstrap-native-prune-saved"),
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
        sleep(Duration::from_millis(5)).await;
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

#[cfg(any(unix, windows))]
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
