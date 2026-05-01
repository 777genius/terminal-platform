use super::{prelude::*, support::*};

#[tokio::test(flavor = "multi_thread")]
async fn fetches_topology_and_screen_for_native_session() {
    let address = unique_address("daemon-client-topology");
    let server = spawn_local_socket_server(TerminalDaemon::default(), address.clone())
        .expect("server should bind");
    let client = LocalSocketDaemonClient::new(address);
    let created = client
        .create_session(
            BackendKind::Native,
            CreateSessionSpec { title: Some("shell".to_string()), ..CreateSessionSpec::default() },
        )
        .await
        .expect("create_session should succeed");
    let topology = client
        .topology_snapshot(created.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let pane_id = topology.tabs[0].focused_pane.expect("focused pane should exist");
    let screen = client
        .screen_snapshot(created.session.session_id, pane_id)
        .await
        .expect("screen_snapshot should succeed");

    assert_eq!(topology.session_id, created.session.session_id);
    assert_eq!(screen.pane_id, pane_id);
    assert!(!screen.surface.lines.is_empty());

    server.shutdown().await.expect("server shutdown should succeed");
}

#[tokio::test(flavor = "multi_thread")]
async fn dispatches_tab_mutations_and_observes_topology_change() {
    let address = unique_address("daemon-client-dispatch");
    let server = spawn_local_socket_server(TerminalDaemon::default(), address.clone())
        .expect("server should bind");
    let client = LocalSocketDaemonClient::new(address);
    let created = client
        .create_session(
            BackendKind::Native,
            CreateSessionSpec { title: Some("shell".to_string()), ..CreateSessionSpec::default() },
        )
        .await
        .expect("create_session should succeed");

    let result = client
        .dispatch(
            created.session.session_id,
            MuxCommand::NewTab(NewTabSpec { title: Some("logs".to_string()) }),
        )
        .await
        .expect("dispatch should succeed");
    let topology = client
        .topology_snapshot(created.session.session_id)
        .await
        .expect("topology snapshot should succeed");

    assert!(result.changed);
    assert_eq!(topology.tabs.len(), 2);
    assert_eq!(topology.focused_tab, Some(topology.tabs[1].tab_id));

    server.shutdown().await.expect("server shutdown should succeed");
}

#[tokio::test(flavor = "multi_thread")]
async fn maps_backend_errors_for_invalid_close_tab_sequence() {
    let address = unique_address("daemon-client-errors");
    let server = spawn_local_socket_server(TerminalDaemon::default(), address.clone())
        .expect("server should bind");
    let client = LocalSocketDaemonClient::new(address);
    let created = client
        .create_session(
            BackendKind::Native,
            CreateSessionSpec { title: Some("shell".to_string()), ..CreateSessionSpec::default() },
        )
        .await
        .expect("create_session should succeed");
    let topology = client
        .topology_snapshot(created.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let only_tab = topology.tabs[0].tab_id;
    let error = client
        .dispatch(created.session.session_id, MuxCommand::CloseTab { tab_id: only_tab })
        .await
        .expect_err("close last tab should fail");

    assert_eq!(error.code, "backend_invalid_input");

    server.shutdown().await.expect("server shutdown should succeed");
}

#[tokio::test(flavor = "multi_thread")]
async fn fetches_screen_delta_for_native_session() {
    let address = unique_address("daemon-client-delta");
    let server = spawn_local_socket_server(TerminalDaemon::default(), address.clone())
        .expect("server should bind");
    let client = LocalSocketDaemonClient::new(address);
    let created = client
        .create_session(
            BackendKind::Native,
            CreateSessionSpec { title: Some("shell".to_string()), ..CreateSessionSpec::default() },
        )
        .await
        .expect("create_session should succeed");
    let topology = client
        .topology_snapshot(created.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let pane_id = topology.tabs[0].focused_pane.expect("focused pane should exist");
    let snapshot = client
        .screen_snapshot(created.session.session_id, pane_id)
        .await
        .expect("screen_snapshot should succeed");
    let delta = client
        .screen_delta(created.session.session_id, pane_id, snapshot.sequence)
        .await
        .expect("screen_delta should succeed");

    assert_eq!(delta.pane_id, pane_id);
    assert_eq!(delta.from_sequence, snapshot.sequence);
    assert_eq!(delta.to_sequence, snapshot.sequence);
    assert_eq!(delta.rows, snapshot.rows);
    assert_eq!(delta.cols, snapshot.cols);
    assert!(delta.patch.is_none());
    assert!(delta.full_replace.is_none());

    server.shutdown().await.expect("server shutdown should succeed");
}

#[tokio::test(flavor = "multi_thread")]
async fn observes_title_only_screen_delta_after_tab_rename() {
    let address = unique_address("daemon-client-title-delta");
    let server = spawn_local_socket_server(TerminalDaemon::default(), address.clone())
        .expect("server should bind");
    let client = LocalSocketDaemonClient::new(address);
    let created = client
        .create_session(
            BackendKind::Native,
            CreateSessionSpec { title: Some("shell".to_string()), ..CreateSessionSpec::default() },
        )
        .await
        .expect("create_session should succeed");
    let topology = client
        .topology_snapshot(created.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let pane_id = topology.tabs[0].focused_pane.expect("focused pane should exist");
    let tab_id = topology.tabs[0].tab_id;
    let before = client
        .screen_snapshot(created.session.session_id, pane_id)
        .await
        .expect("screen_snapshot should succeed");

    let result = client
        .dispatch(
            created.session.session_id,
            MuxCommand::RenameTab { tab_id, title: "renamed".to_string() },
        )
        .await
        .expect("rename tab should succeed");
    let delta = client
        .screen_delta(created.session.session_id, pane_id, before.sequence)
        .await
        .expect("screen_delta should succeed");
    let listed = client.list_sessions().await.expect("list_sessions should succeed");
    let patch = delta.patch.expect("delta patch should exist");

    assert!(result.changed);
    assert_eq!(listed.sessions[0].title.as_deref(), Some("renamed"));
    assert!(delta.to_sequence > before.sequence);
    assert!(patch.title_changed);
    assert_eq!(patch.title.as_deref(), Some("renamed"));
    assert!(patch.line_updates.is_empty());
    assert!(delta.full_replace.is_none());

    server.shutdown().await.expect("server shutdown should succeed");
}
