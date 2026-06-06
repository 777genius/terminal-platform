use super::super::{prelude::*, support::*};

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_smoke_discovers_and_imports_tmux_session() {
    let socket_name = unique_tmux_socket_name("bootstrap-tmux");
    let session_name = unique_tmux_session_name("workspace");
    let _tmux =
        TmuxServerGuard::spawn(&socket_name, &session_name).expect("tmux test server should start");
    let fixture = daemon_fixture_with_daemon("bootstrap-tmux-import", tmux_daemon(&socket_name))
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
                client_event_id: None,
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
        daemon_fixture_with_daemon("bootstrap-tmux-inactive-pane", tmux_daemon(&socket_name))
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
