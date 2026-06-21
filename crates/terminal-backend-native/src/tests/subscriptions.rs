use terminal_backend_api::{
    BackendSubscriptionEvent, CreateSessionSpec, MuxBackendPort, MuxCommand, NewTabSpec,
    SubscriptionSpec,
};
#[cfg(any(unix, windows))]
use terminal_backend_api::{ResizePaneSpec, SplitPaneSpec};
#[cfg(any(unix, windows))]
use terminal_mux_domain::SplitDirection;

use crate::NativeBackend;

#[cfg(any(unix, windows))]
use super::support::{cat_launch_spec, collect_pane_ids, quiet_launch_spec, wait_for_screen_line};

#[tokio::test]
async fn streams_initial_topology_and_new_tab_updates() {
    let backend = NativeBackend::default();
    let binding = backend
        .create_session(CreateSessionSpec {
            title: Some("shell".to_string()),
            ..CreateSessionSpec::default()
        })
        .await
        .expect("native session should be created");
    let session = backend
        .attach_session(binding.session_id, binding.route)
        .await
        .expect("attach_session should succeed");
    let mut subscription = session
        .subscribe(SubscriptionSpec::SessionTopology)
        .await
        .expect("topology subscription should open");

    let initial = subscription.events.recv().await.expect("initial event should arrive");
    let initial = match initial {
        BackendSubscriptionEvent::TopologySnapshot(snapshot) => snapshot,
        other => panic!("unexpected initial event: {other:?}"),
    };
    let result = session
        .dispatch(MuxCommand::NewTab(NewTabSpec { title: Some("logs".to_string()) }))
        .await
        .expect("new tab should succeed");
    let updated = subscription.events.recv().await.expect("topology update should arrive");
    let updated = match updated {
        BackendSubscriptionEvent::TopologySnapshot(snapshot) => snapshot,
        other => panic!("unexpected topology event: {other:?}"),
    };

    assert_eq!(initial.tabs.len(), 1);
    assert!(result.changed);
    assert_eq!(updated.tabs.len(), 2);
}

#[tokio::test]
async fn streams_initial_surface_and_title_patch_updates() {
    let backend = NativeBackend::default();
    let binding = backend
        .create_session(CreateSessionSpec {
            title: Some("shell".to_string()),
            launch: Some(quiet_launch_spec()),
        })
        .await
        .expect("native session should be created");
    let session = backend
        .attach_session(binding.session_id, binding.route)
        .await
        .expect("attach_session should succeed");
    let topology = session.topology_snapshot().await.expect("topology should succeed");
    let pane_id = topology.tabs[0].focused_pane.expect("focused pane should exist");
    let tab_id = topology.tabs[0].tab_id;
    let mut subscription = session
        .subscribe(SubscriptionSpec::PaneSurface { pane_id })
        .await
        .expect("pane subscription should open");

    let initial = subscription.events.recv().await.expect("initial event should arrive");
    let initial = match initial {
        BackendSubscriptionEvent::ScreenDelta(delta) => *delta,
        other => panic!("unexpected initial event: {other:?}"),
    };
    let result = session
        .dispatch(MuxCommand::RenameTab { tab_id, title: "renamed".to_string() })
        .await
        .expect("rename tab should succeed");
    let updated = subscription.events.recv().await.expect("surface update should arrive");
    let updated = match updated {
        BackendSubscriptionEvent::ScreenDelta(delta) => *delta,
        other => panic!("unexpected surface event: {other:?}"),
    };
    let patch = updated.patch.expect("delta patch should exist");

    assert!(initial.full_replace.is_some());
    assert!(result.changed);
    assert!(updated.to_sequence > updated.from_sequence);
    assert!(patch.title_changed);
    assert_eq!(patch.title.as_deref(), Some("renamed"));
}

#[cfg(any(unix, windows))]
#[tokio::test]
async fn streams_surface_updates_for_all_affected_panes_after_resize() {
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
    let topology = session.topology_snapshot().await.expect("topology should succeed");
    let original_pane = topology.tabs[0].focused_pane.expect("focused pane should exist");

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
    let resized_pane = pane_ids
        .iter()
        .copied()
        .find(|candidate| *candidate != original_pane)
        .expect("new pane should exist");
    wait_for_screen_line(&*session, resized_pane, "ready").await;

    let mut original_subscription = session
        .subscribe(SubscriptionSpec::PaneSurface { pane_id: original_pane })
        .await
        .expect("original pane subscription should open");
    let mut resized_subscription = session
        .subscribe(SubscriptionSpec::PaneSurface { pane_id: resized_pane })
        .await
        .expect("resized pane subscription should open");

    let original_initial =
        match original_subscription.events.recv().await.expect("initial event should arrive") {
            BackendSubscriptionEvent::ScreenDelta(delta) => *delta,
            other => panic!("unexpected initial original event: {other:?}"),
        };
    let resized_initial =
        match resized_subscription.events.recv().await.expect("initial event should arrive") {
            BackendSubscriptionEvent::ScreenDelta(delta) => *delta,
            other => panic!("unexpected initial resized event: {other:?}"),
        };

    let resized_before =
        session.screen_snapshot(resized_pane).await.expect("screen snapshot should succeed");
    let total_cols = session
        .screen_snapshot(original_pane)
        .await
        .expect("screen snapshot should succeed")
        .cols
        .saturating_add(resized_before.cols);
    let target_cols = resized_before.cols.saturating_add(10).min(total_cols.saturating_sub(1));
    let resize = session
        .dispatch(MuxCommand::ResizePane(ResizePaneSpec {
            pane_id: resized_pane,
            rows: resized_before.rows,
            cols: target_cols,
        }))
        .await
        .expect("resize should succeed");

    let original_updated =
        match original_subscription.events.recv().await.expect("updated event should arrive") {
            BackendSubscriptionEvent::ScreenDelta(delta) => *delta,
            other => panic!("unexpected original update event: {other:?}"),
        };
    let resized_updated =
        match resized_subscription.events.recv().await.expect("updated event should arrive") {
            BackendSubscriptionEvent::ScreenDelta(delta) => *delta,
            other => panic!("unexpected resized update event: {other:?}"),
        };

    assert!(original_initial.full_replace.is_some());
    assert!(resized_initial.full_replace.is_some());
    assert!(resize.changed);
    assert_eq!(original_updated.pane_id, original_pane);
    assert_eq!(resized_updated.pane_id, resized_pane);
    assert!(original_updated.full_replace.is_some());
    assert!(resized_updated.full_replace.is_some());
}
