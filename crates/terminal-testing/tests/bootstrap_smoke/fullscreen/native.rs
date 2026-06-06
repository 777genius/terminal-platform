use super::{super::prelude::*, helpers::*};

#[cfg(windows)]
#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_smoke_preserves_native_fullscreen_viewports_for_vim_less_and_fzf() {
    if !fullscreen_tools_available() {
        return;
    }

    let fixture = daemon_fixture("bootstrap-native-fullscreen").expect("fixture should start");
    let created = fixture
        .client
        .create_session(
            BackendKind::Native,
            CreateSessionSpec { title: Some("shell".to_string()), launch: None },
        )
        .await
        .expect("native fullscreen session should create");
    let topology = fixture
        .client
        .topology_snapshot(created.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let focused_pane = topology.tabs[0].focused_pane.expect("focused pane should exist");

    wait_for_shell_marker(&fixture, created.session.session_id, focused_pane, "native-initial")
        .await;
    run_fullscreen_viewport_flow(
        &fixture,
        created.session.session_id,
        focused_pane,
        "native",
        true,
        true,
    )
    .await;

    fixture.shutdown().await.expect("fixture should stop cleanly");
}
