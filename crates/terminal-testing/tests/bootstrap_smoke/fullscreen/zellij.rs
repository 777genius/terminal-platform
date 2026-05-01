use super::{
    super::{prelude::*, support::*},
    helpers::*,
};

#[cfg(any(unix, windows))]
#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_smoke_preserves_zellij_fullscreen_viewports_for_imported_tuis() {
    if !fullscreen_tools_available() {
        return;
    }

    let session_name = unique_zellij_session_name("full");
    let _zellij = ZellijSessionGuard::spawn(&session_name).expect("zellij session should start");
    let fixture = daemon_fixture("bootstrap-zellij-fullscreen").expect("fixture should start");
    let capabilities = fixture
        .client
        .backend_capabilities(BackendKind::Zellij)
        .await
        .expect("zellij capabilities should succeed");

    if !capabilities.capabilities.rendered_viewport_snapshot {
        fixture.shutdown().await.expect("fixture should stop cleanly");
        return;
    }

    let candidate = wait_for_discovered_zellij_session(&fixture.client, &session_name).await;
    let imported = tokio::time::timeout(
        zellij_operation_timeout(),
        fixture.client.import_session(candidate.route, candidate.title),
    )
    .await
    .expect("zellij import should not hang")
    .expect("zellij import should succeed");
    let topology = wait_for_topology(
        &fixture,
        imported.session.session_id,
        |snapshot| !snapshot.tabs.is_empty(),
        "zellij fullscreen initial topology",
    )
    .await;
    let focused_tab = topology
        .tabs
        .iter()
        .find(|tab| Some(tab.tab_id) == topology.focused_tab)
        .or_else(|| topology.tabs.first())
        .expect("zellij fullscreen tab should exist");
    let focused_pane = focused_tab
        .focused_pane
        .or_else(|| collect_pane_ids(&focused_tab.root).first().copied())
        .expect("zellij fullscreen pane should exist");

    wait_for_shell_marker(&fixture, imported.session.session_id, focused_pane, "zellij-initial")
        .await;
    // Imported Unix Zellij sessions do not expose a truthful automated proof source for plain
    // `less` through `dump-screen`, so keep automated parity honest there and leave pager
    // validation to the documented manual `less -X` acceptance path.
    let exercise_less = cfg!(windows);
    // Hosted Windows import coverage still cannot prove `fzf` viewport fidelity through the
    // imported Zellij screen path without lying about parity, so keep that specific proof manual.
    let exercise_fzf = !cfg!(windows);
    run_fullscreen_viewport_flow(
        &fixture,
        imported.session.session_id,
        focused_pane,
        "zellij",
        exercise_less,
        exercise_fzf,
    )
    .await;

    fixture.shutdown().await.expect("fixture should stop cleanly");
}
