use super::super::{prelude::*, support::*};

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_smoke_controls_tmux_tab_lifecycle_via_dispatch() {
    let socket_name = unique_tmux_socket_name("bootstrap-tmux-tabs");
    let session_name = unique_tmux_session_name("workspace");
    let _tmux =
        TmuxServerGuard::spawn(&socket_name, &session_name).expect("tmux test server should start");
    let fixture =
        daemon_fixture_with_daemon("bootstrap-tmux-tab-control", tmux_daemon(&socket_name))
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
    let initial = fixture
        .client
        .topology_snapshot(imported.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let initial_focused_tab = initial.focused_tab.expect("focused tab should exist");

    let created = fixture
        .client
        .dispatch(
            imported.session.session_id,
            MuxCommand::NewTab(NewTabSpec { title: Some("metrics".to_string()) }),
        )
        .await
        .expect("new tab should succeed");
    let after_create = fixture
        .client
        .topology_snapshot(imported.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let metrics_tab = after_create
        .tabs
        .iter()
        .find(|tab| tab.title.as_deref() == Some("metrics"))
        .map(|tab| tab.tab_id)
        .expect("created tab should exist");

    let focused = fixture
        .client
        .dispatch(imported.session.session_id, MuxCommand::FocusTab { tab_id: initial_focused_tab })
        .await
        .expect("focus tab should succeed");
    let after_focus = fixture
        .client
        .topology_snapshot(imported.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let closed = fixture
        .client
        .dispatch(imported.session.session_id, MuxCommand::CloseTab { tab_id: metrics_tab })
        .await
        .expect("close tab should succeed");
    let after_close = fixture
        .client
        .topology_snapshot(imported.session.session_id)
        .await
        .expect("topology_snapshot should succeed");

    assert!(created.changed);
    assert_eq!(after_create.tabs.len(), 3);
    assert_eq!(after_create.focused_tab, Some(metrics_tab));
    assert!(focused.changed);
    assert_eq!(after_focus.focused_tab, Some(initial_focused_tab));
    assert!(closed.changed);
    assert_eq!(after_close.tabs.len(), 2);
    assert!(after_close.tabs.iter().all(|tab| tab.tab_id != metrics_tab));

    fixture.shutdown().await.expect("fixture should stop cleanly");
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_smoke_controls_tmux_pane_lifecycle_via_dispatch() {
    let socket_name = unique_tmux_socket_name("bootstrap-tmux-panes");
    let session_name = unique_tmux_session_name("workspace");
    let _tmux =
        TmuxServerGuard::spawn(&socket_name, &session_name).expect("tmux test server should start");
    let fixture =
        daemon_fixture_with_daemon("bootstrap-tmux-pane-control", tmux_daemon(&socket_name))
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
    let initial = fixture
        .client
        .topology_snapshot(imported.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let focused_tab = initial.focused_tab.expect("focused tab should exist");
    let focused_tab_snapshot = initial
        .tabs
        .iter()
        .find(|tab| tab.tab_id == focused_tab)
        .expect("focused tab snapshot should exist");
    let focused_pane = focused_tab_snapshot.focused_pane.expect("focused pane should exist");
    let initial_panes = collect_pane_ids(&focused_tab_snapshot.root);

    let split = fixture
        .client
        .dispatch(
            imported.session.session_id,
            MuxCommand::SplitPane(SplitPaneSpec {
                pane_id: focused_pane,
                direction: SplitDirection::Vertical,
            }),
        )
        .await
        .expect("split pane should succeed");
    let after_split = fixture
        .client
        .topology_snapshot(imported.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let split_tab = after_split
        .tabs
        .iter()
        .find(|tab| tab.tab_id == focused_tab)
        .expect("split tab snapshot should exist");
    let split_panes = collect_pane_ids(&split_tab.root);
    let new_pane = split_panes
        .iter()
        .copied()
        .find(|pane_id| !initial_panes.contains(pane_id))
        .expect("new pane should exist after split");
    let before_resize = fixture
        .client
        .screen_snapshot(imported.session.session_id, new_pane)
        .await
        .expect("screen_snapshot should succeed");
    let resize = fixture
        .client
        .dispatch(
            imported.session.session_id,
            MuxCommand::ResizePane(ResizePaneSpec {
                pane_id: new_pane,
                rows: before_resize.rows.saturating_sub(4).max(4),
                cols: before_resize.cols.saturating_sub(6).max(10),
            }),
        )
        .await
        .expect("resize pane should succeed");
    let after_resize = fixture
        .client
        .screen_snapshot(imported.session.session_id, new_pane)
        .await
        .expect("screen_snapshot should succeed");

    let focus = fixture
        .client
        .dispatch(imported.session.session_id, MuxCommand::FocusPane { pane_id: focused_pane })
        .await
        .expect("focus pane should succeed");
    let after_focus = fixture
        .client
        .topology_snapshot(imported.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let focused_tab_after_focus = after_focus
        .tabs
        .iter()
        .find(|tab| tab.tab_id == focused_tab)
        .expect("focused tab snapshot should exist");
    let close = fixture
        .client
        .dispatch(imported.session.session_id, MuxCommand::ClosePane { pane_id: new_pane })
        .await
        .expect("close pane should succeed");
    let after_close = fixture
        .client
        .topology_snapshot(imported.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let focused_tab_after_close = after_close
        .tabs
        .iter()
        .find(|tab| tab.tab_id == focused_tab)
        .expect("focused tab snapshot should exist");
    let single_pane = after_close
        .tabs
        .iter()
        .find(|tab| tab.tab_id != focused_tab && collect_pane_ids(&tab.root).len() == 1)
        .and_then(|tab| collect_pane_ids(&tab.root).into_iter().next())
        .expect("single-pane secondary tab should exist");
    let close_last_error = fixture
        .client
        .dispatch(imported.session.session_id, MuxCommand::ClosePane { pane_id: single_pane })
        .await
        .expect_err("closing last pane in tab should be rejected");

    assert!(split.changed);
    assert_eq!(split_panes.len(), initial_panes.len() + 1);
    assert_eq!(split_tab.focused_pane, Some(new_pane));
    assert!(resize.changed);
    assert!(
        after_resize.rows != before_resize.rows || after_resize.cols != before_resize.cols,
        "resize should change at least one pane dimension"
    );
    assert!(focus.changed);
    assert_eq!(focused_tab_after_focus.focused_pane, Some(focused_pane));
    assert!(close.changed);
    assert_eq!(collect_pane_ids(&focused_tab_after_close.root).len(), initial_panes.len());
    assert_eq!(close_last_error.code, "backend_unsupported");
    assert_eq!(close_last_error.degraded_reason, Some(DegradedModeReason::UnsupportedByBackend));

    fixture.shutdown().await.expect("fixture should stop cleanly");
}
