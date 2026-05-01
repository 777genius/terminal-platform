use terminal_backend_api::{
    BackendErrorKind, CreateSessionSpec, MuxBackendPort, MuxCommand, OverrideLayoutSpec,
    ResizePaneSpec, SplitPaneSpec,
};
use terminal_mux_domain::{PaneSplit, PaneTreeNode, SplitDirection};

use crate::NativeBackend;

use super::support::{cat_launch_spec, collect_pane_ids, wait_for_screen_line};

#[tokio::test]
async fn splits_and_closes_panes_within_native_tab() {
    let backend = NativeBackend::default();
    let binding = backend
        .create_session(CreateSessionSpec {
            title: Some("shell".to_string()),
            launch: Some(cat_launch_spec()),
        })
        .await
        .expect("native session should be created");
    let session = backend
        .attach_session(binding.session_id, binding.route)
        .await
        .expect("attach_session should succeed");
    let initial = session.topology_snapshot().await.expect("topology should succeed");
    let tab = &initial.tabs[0];
    let pane_id = tab.focused_pane.expect("focused pane should exist");
    wait_for_screen_line(&*session, pane_id, "ready").await;
    let initial_screen =
        session.screen_snapshot(pane_id).await.expect("screen snapshot should succeed");

    let split = session
        .dispatch(MuxCommand::SplitPane(SplitPaneSpec {
            pane_id,
            direction: SplitDirection::Vertical,
        }))
        .await
        .expect("split pane should succeed");
    let after_split = session.topology_snapshot().await.expect("topology should succeed");
    let split_tab = &after_split.tabs[0];
    let pane_ids = collect_pane_ids(&split_tab.root);
    let new_pane = pane_ids
        .iter()
        .copied()
        .find(|candidate| *candidate != pane_id)
        .expect("new pane should exist");

    wait_for_screen_line(&*session, new_pane, "ready").await;
    let original_after_split =
        session.screen_snapshot(pane_id).await.expect("screen snapshot should succeed");
    let new_after_split =
        session.screen_snapshot(new_pane).await.expect("screen snapshot should succeed");
    let close = session
        .dispatch(MuxCommand::ClosePane { pane_id: new_pane })
        .await
        .expect("close pane should succeed");
    let after_close = session.topology_snapshot().await.expect("topology should succeed");
    let restored_screen =
        session.screen_snapshot(pane_id).await.expect("screen snapshot should succeed");
    let close_last = session
        .dispatch(MuxCommand::ClosePane { pane_id })
        .await
        .expect_err("closing last pane should fail");

    assert!(split.changed);
    assert_eq!(pane_ids.len(), 2);
    assert_eq!(split_tab.focused_pane, Some(new_pane));
    assert_eq!(original_after_split.rows, initial_screen.rows);
    assert_eq!(new_after_split.rows, initial_screen.rows);
    assert!(original_after_split.cols < initial_screen.cols);
    assert!(new_after_split.cols < initial_screen.cols);
    assert!(close.changed);
    assert_eq!(collect_pane_ids(&after_close.tabs[0].root), vec![pane_id]);
    assert_eq!(restored_screen.rows, initial_screen.rows);
    assert_eq!(restored_screen.cols, initial_screen.cols);
    assert_eq!(close_last.kind, BackendErrorKind::InvalidInput);
}

#[tokio::test]
async fn resizes_split_panes_through_layout_ratios() {
    let backend = NativeBackend::default();
    let binding = backend
        .create_session(CreateSessionSpec {
            title: Some("shell".to_string()),
            launch: Some(cat_launch_spec()),
        })
        .await
        .expect("native session should be created");
    let session = backend
        .attach_session(binding.session_id, binding.route)
        .await
        .expect("attach_session should succeed");
    let initial = session.topology_snapshot().await.expect("topology should succeed");
    let pane_id = initial.tabs[0].focused_pane.expect("focused pane should exist");

    wait_for_screen_line(&*session, pane_id, "ready").await;
    session
        .dispatch(MuxCommand::SplitPane(SplitPaneSpec {
            pane_id,
            direction: SplitDirection::Vertical,
        }))
        .await
        .expect("split pane should succeed");
    let after_split = session.topology_snapshot().await.expect("topology should succeed");
    let pane_ids = collect_pane_ids(&after_split.tabs[0].root);
    let new_pane = pane_ids
        .iter()
        .copied()
        .find(|candidate| *candidate != pane_id)
        .expect("new pane should exist");

    wait_for_screen_line(&*session, new_pane, "ready").await;
    let original_before =
        session.screen_snapshot(pane_id).await.expect("screen snapshot should succeed");
    let target_before =
        session.screen_snapshot(new_pane).await.expect("screen snapshot should succeed");

    let resize_row = session
        .dispatch(MuxCommand::ResizePane(ResizePaneSpec {
            pane_id: new_pane,
            rows: target_before.rows.saturating_sub(4).max(4),
            cols: target_before.cols,
        }))
        .await
        .expect_err("row resize should be rejected without horizontal split authority");
    let total_cols = original_before.cols.saturating_add(target_before.cols);
    let target_cols = target_before.cols.saturating_add(10).min(total_cols.saturating_sub(1));
    let resize = session
        .dispatch(MuxCommand::ResizePane(ResizePaneSpec {
            pane_id: new_pane,
            rows: target_before.rows,
            cols: target_cols,
        }))
        .await
        .expect("col resize should succeed");
    let original_after =
        session.screen_snapshot(pane_id).await.expect("screen snapshot should succeed");
    let target_after =
        session.screen_snapshot(new_pane).await.expect("screen snapshot should succeed");

    assert_eq!(resize_row.kind, BackendErrorKind::Unsupported);
    assert!(resize.changed);
    assert_eq!(target_after.rows, target_before.rows);
    assert_eq!(original_after.rows, original_before.rows);
    assert!(target_after.cols > target_before.cols);
    assert!(original_after.cols < original_before.cols);
    assert_eq!(target_after.cols + original_after.cols, target_before.cols + original_before.cols);
}

#[tokio::test]
async fn overrides_native_layout_with_existing_pane_set() {
    let backend = NativeBackend::default();
    let binding = backend
        .create_session(CreateSessionSpec {
            title: Some("shell".to_string()),
            launch: Some(cat_launch_spec()),
        })
        .await
        .expect("native session should be created");
    let session = backend
        .attach_session(binding.session_id, binding.route)
        .await
        .expect("attach_session should succeed");
    let initial = session.topology_snapshot().await.expect("topology should succeed");
    let tab_id = initial.tabs[0].tab_id;
    let original_pane = initial.tabs[0].focused_pane.expect("focused pane should exist");

    wait_for_screen_line(&*session, original_pane, "ready").await;
    session
        .dispatch(MuxCommand::SplitPane(SplitPaneSpec {
            pane_id: original_pane,
            direction: SplitDirection::Vertical,
        }))
        .await
        .expect("split pane should succeed");
    let after_split = session.topology_snapshot().await.expect("topology should succeed");
    let pane_ids = collect_pane_ids(&after_split.tabs[0].root);
    let new_pane = pane_ids
        .iter()
        .copied()
        .find(|candidate| *candidate != original_pane)
        .expect("new pane should exist");
    wait_for_screen_line(&*session, new_pane, "ready").await;

    let original_before =
        session.screen_snapshot(original_pane).await.expect("screen snapshot should succeed");
    let new_before =
        session.screen_snapshot(new_pane).await.expect("screen snapshot should succeed");
    let override_layout = PaneTreeNode::Split(PaneSplit {
        direction: SplitDirection::Horizontal,
        first: Box::new(PaneTreeNode::Leaf { pane_id: original_pane }),
        second: Box::new(PaneTreeNode::Leaf { pane_id: new_pane }),
    });
    let override_result = session
        .dispatch(MuxCommand::OverrideLayout(OverrideLayoutSpec {
            tab_id,
            root: override_layout.clone(),
        }))
        .await
        .expect("layout override should succeed");
    let invalid_override = session
        .dispatch(MuxCommand::OverrideLayout(OverrideLayoutSpec {
            tab_id,
            root: PaneTreeNode::Split(PaneSplit {
                direction: SplitDirection::Horizontal,
                first: Box::new(PaneTreeNode::Leaf { pane_id: original_pane }),
                second: Box::new(PaneTreeNode::Leaf { pane_id: original_pane }),
            }),
        }))
        .await
        .expect_err("duplicate pane ids should be rejected");
    let after_override = session.topology_snapshot().await.expect("topology should succeed");
    let original_after =
        session.screen_snapshot(original_pane).await.expect("screen snapshot should succeed");
    let new_after =
        session.screen_snapshot(new_pane).await.expect("screen snapshot should succeed");

    assert!(override_result.changed);
    assert_eq!(after_override.tabs[0].root, override_layout);
    assert!(original_after.rows < original_before.rows);
    assert!(new_after.rows < new_before.rows);
    assert!(original_after.cols > original_before.cols);
    assert!(new_after.cols > new_before.cols);
    assert_eq!(invalid_override.kind, BackendErrorKind::InvalidInput);
}
