use super::super::{prelude::*, support::*};

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_smoke_controls_native_pane_lifecycle_via_dispatch() {
    let fixture = daemon_fixture("bootstrap-native-pane-control").expect("fixture should start");
    let created = fixture
        .client
        .create_session(
            BackendKind::Native,
            CreateSessionSpec { title: Some("shell".to_string()), launch: Some(cat_launch_spec()) },
        )
        .await
        .expect("create_session should succeed");
    let initial = fixture
        .client
        .topology_snapshot(created.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let focused_tab = initial.focused_tab.expect("focused tab should exist");
    let tab = initial
        .tabs
        .iter()
        .find(|tab| tab.tab_id == focused_tab)
        .expect("focused tab should exist");
    let pane_id = tab.focused_pane.expect("focused pane should exist");
    wait_for_screen_line(&fixture, created.session.session_id, pane_id, "ready").await;
    let initial_screen = fixture
        .client
        .screen_snapshot(created.session.session_id, pane_id)
        .await
        .expect("screen_snapshot should succeed");

    let split = fixture
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
    let split_tab = after_split
        .tabs
        .iter()
        .find(|tab| tab.tab_id == focused_tab)
        .expect("split tab should exist");
    let pane_ids = collect_pane_ids(&split_tab.root);
    let new_pane = pane_ids
        .iter()
        .copied()
        .find(|candidate| *candidate != pane_id)
        .expect("new pane should exist");

    wait_for_screen_line(&fixture, created.session.session_id, new_pane, "ready").await;
    let original_after_split = fixture
        .client
        .screen_snapshot(created.session.session_id, pane_id)
        .await
        .expect("screen_snapshot should succeed");
    let new_after_split = fixture
        .client
        .screen_snapshot(created.session.session_id, new_pane)
        .await
        .expect("screen_snapshot should succeed");
    let focus = fixture
        .client
        .dispatch(created.session.session_id, MuxCommand::FocusPane { pane_id })
        .await
        .expect("focus pane should succeed");
    let close = fixture
        .client
        .dispatch(created.session.session_id, MuxCommand::ClosePane { pane_id: new_pane })
        .await
        .expect("close pane should succeed");
    let after_close = fixture
        .client
        .topology_snapshot(created.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let restored_screen = fixture
        .client
        .screen_snapshot(created.session.session_id, pane_id)
        .await
        .expect("screen_snapshot should succeed");
    let close_last = fixture
        .client
        .dispatch(created.session.session_id, MuxCommand::ClosePane { pane_id })
        .await
        .expect_err("closing last pane should fail");

    assert!(split.changed);
    assert_eq!(split_tab.focused_pane, Some(new_pane));
    assert_eq!(original_after_split.rows, initial_screen.rows);
    assert_eq!(new_after_split.rows, initial_screen.rows);
    assert!(original_after_split.cols < initial_screen.cols);
    assert!(new_after_split.cols < initial_screen.cols);
    assert!(focus.changed);
    assert!(close.changed);
    assert_eq!(collect_pane_ids(&after_close.tabs[0].root), vec![pane_id]);
    assert_eq!(restored_screen.rows, initial_screen.rows);
    assert_eq!(restored_screen.cols, initial_screen.cols);
    assert_eq!(close_last.code, "backend_invalid_input");

    fixture.shutdown().await.expect("fixture should stop cleanly");
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_smoke_resizes_native_split_panes_through_layout_ratios() {
    let fixture = daemon_fixture("bootstrap-native-pane-resize").expect("fixture should start");
    let created = fixture
        .client
        .create_session(
            BackendKind::Native,
            CreateSessionSpec { title: Some("shell".to_string()), launch: Some(cat_launch_spec()) },
        )
        .await
        .expect("create_session should succeed");
    let initial = fixture
        .client
        .topology_snapshot(created.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let pane_id = initial.tabs[0].focused_pane.expect("focused pane should exist");

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
    let original_before = fixture
        .client
        .screen_snapshot(created.session.session_id, pane_id)
        .await
        .expect("screen_snapshot should succeed");
    let target_before = fixture
        .client
        .screen_snapshot(created.session.session_id, new_pane)
        .await
        .expect("screen_snapshot should succeed");

    let resize_row = fixture
        .client
        .dispatch(
            created.session.session_id,
            MuxCommand::ResizePane(ResizePaneSpec {
                pane_id: new_pane,
                rows: target_before.rows.saturating_sub(4).max(4),
                cols: target_before.cols,
            }),
        )
        .await
        .expect_err("row resize should be rejected without horizontal split authority");
    let total_cols = original_before.cols.saturating_add(target_before.cols);
    let target_cols = target_before.cols.saturating_add(10).min(total_cols.saturating_sub(1));
    let resize = fixture
        .client
        .dispatch(
            created.session.session_id,
            MuxCommand::ResizePane(ResizePaneSpec {
                pane_id: new_pane,
                rows: target_before.rows,
                cols: target_cols,
            }),
        )
        .await
        .expect("col resize should succeed");
    let original_after = fixture
        .client
        .screen_snapshot(created.session.session_id, pane_id)
        .await
        .expect("screen_snapshot should succeed");
    let target_after = fixture
        .client
        .screen_snapshot(created.session.session_id, new_pane)
        .await
        .expect("screen_snapshot should succeed");

    assert_eq!(resize_row.code, "backend_unsupported");
    assert!(resize.changed);
    assert_eq!(target_after.rows, target_before.rows);
    assert_eq!(original_after.rows, original_before.rows);
    assert!(target_after.cols > target_before.cols);
    assert!(original_after.cols < original_before.cols);
    assert_eq!(target_after.cols + original_after.cols, target_before.cols + original_before.cols);

    fixture.shutdown().await.expect("fixture should stop cleanly");
}
