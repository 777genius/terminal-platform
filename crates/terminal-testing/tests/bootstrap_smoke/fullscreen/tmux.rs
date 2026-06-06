use super::{
    super::{prelude::*, support::*},
    helpers::*,
};

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_smoke_preserves_tmux_fullscreen_viewports_for_vim_less_and_fzf() {
    if !fullscreen_tools_available() {
        return;
    }

    let socket_name = unique_tmux_socket_name("bootstrap-tmux-fullscreen");
    let session_name = unique_tmux_session_name("workspace");
    let _tmux = TmuxServerGuard::spawn_with_shell(&socket_name, &session_name)
        .expect("tmux interactive test server should start");
    let fixture =
        daemon_fixture_with_daemon("bootstrap-tmux-fullscreen", tmux_daemon(&socket_name))
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
    run_fullscreen_viewport_flow(
        &fixture,
        imported.session.session_id,
        focused_pane,
        "tmux",
        true,
        true,
    )
    .await;

    fixture.shutdown().await.expect("fixture should stop cleanly");
}
