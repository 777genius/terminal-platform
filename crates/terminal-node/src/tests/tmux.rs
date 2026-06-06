use super::{prelude::*, support::*};

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn discovers_and_imports_tmux_sessions_through_node_surface() {
    let socket_name = unique_tmux_socket_name("terminal-node-tmux");
    let session_name = unique_tmux_session_name("workspace");
    let _tmux =
        TmuxServerGuard::spawn(&socket_name, &session_name).expect("tmux server should start");
    let fixture = daemon_fixture_with_daemon("terminal-node-tmux", tmux_daemon(&socket_name))
        .expect("fixture should start");
    let node = NodeHostClient::new(fixture.client.address().clone());

    let discovered = node
        .discover_sessions(NodeBackendKind::Tmux)
        .await
        .expect("discover_sessions should succeed");
    let candidate = discovered.first().expect("tmux session should be discoverable").clone();
    let imported = node
        .import_session(&candidate.route, candidate.title.clone())
        .await
        .expect("import_session should succeed");
    let topology = node
        .topology_snapshot(&imported.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let focused_pane = topology
        .tabs
        .iter()
        .find(|tab| Some(tab.tab_id.as_str()) == topology.focused_tab.as_deref())
        .and_then(|tab| tab.focused_pane.clone())
        .expect("focused pane should exist");
    let screen =
        wait_for_screen_line(&node, &imported.session_id, &focused_pane, "hello from tmux").await;

    assert_eq!(candidate.route.backend, NodeBackendKind::Tmux);
    assert_eq!(imported.route.backend, NodeBackendKind::Tmux);
    assert_eq!(topology.backend_kind, NodeBackendKind::Tmux);
    assert_eq!(topology.tabs.len(), 2);
    assert!(screen.surface.lines.iter().any(|line| line.text.contains("hello from tmux")));

    fixture.shutdown().await.expect("fixture should stop cleanly");
}
