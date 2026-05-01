use super::super::{prelude::*, support::*};

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_smoke_overrides_native_layout_via_dispatch() {
    let fixture = daemon_fixture("bootstrap-native-layout-override").expect("fixture should start");
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
    let tab_id = initial.tabs[0].tab_id;
    let original_pane = initial.tabs[0].focused_pane.expect("focused pane should exist");

    wait_for_screen_line(&fixture, created.session.session_id, original_pane, "ready").await;
    fixture
        .client
        .dispatch(
            created.session.session_id,
            MuxCommand::SplitPane(SplitPaneSpec {
                pane_id: original_pane,
                direction: SplitDirection::Vertical,
            }),
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
        .find(|candidate| *candidate != original_pane)
        .expect("new pane should exist");
    wait_for_screen_line(&fixture, created.session.session_id, new_pane, "ready").await;

    let original_before = fixture
        .client
        .screen_snapshot(created.session.session_id, original_pane)
        .await
        .expect("screen_snapshot should succeed");
    let new_before = fixture
        .client
        .screen_snapshot(created.session.session_id, new_pane)
        .await
        .expect("screen_snapshot should succeed");
    let override_layout = PaneTreeNode::Split(PaneSplit {
        direction: SplitDirection::Horizontal,
        first: Box::new(PaneTreeNode::Leaf { pane_id: original_pane }),
        second: Box::new(PaneTreeNode::Leaf { pane_id: new_pane }),
    });
    let override_result = fixture
        .client
        .dispatch(
            created.session.session_id,
            MuxCommand::OverrideLayout(OverrideLayoutSpec {
                tab_id,
                root: override_layout.clone(),
            }),
        )
        .await
        .expect("layout override should succeed");
    let invalid_override = fixture
        .client
        .dispatch(
            created.session.session_id,
            MuxCommand::OverrideLayout(OverrideLayoutSpec {
                tab_id,
                root: PaneTreeNode::Split(PaneSplit {
                    direction: SplitDirection::Horizontal,
                    first: Box::new(PaneTreeNode::Leaf { pane_id: original_pane }),
                    second: Box::new(PaneTreeNode::Leaf { pane_id: original_pane }),
                }),
            }),
        )
        .await
        .expect_err("duplicate pane ids should be rejected");
    let after_override = fixture
        .client
        .topology_snapshot(created.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let original_after = fixture
        .client
        .screen_snapshot(created.session.session_id, original_pane)
        .await
        .expect("screen_snapshot should succeed");
    let new_after = fixture
        .client
        .screen_snapshot(created.session.session_id, new_pane)
        .await
        .expect("screen_snapshot should succeed");

    assert!(override_result.changed);
    assert_eq!(after_override.tabs[0].root, override_layout);
    assert!(original_after.rows < original_before.rows);
    assert!(new_after.rows < new_before.rows);
    assert!(original_after.cols > original_before.cols);
    assert!(new_after.cols > new_before.cols);
    assert_eq!(invalid_override.code, "backend_invalid_input");

    fixture.shutdown().await.expect("fixture should stop cleanly");
}
