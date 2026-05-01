use super::super::{prelude::*, support::*};

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_smoke_streams_live_pane_surface_updates() {
    let fixture = daemon_fixture("bootstrap-pane-sub").expect("fixture should start");
    let created = fixture
        .client
        .create_session(
            BackendKind::Native,
            CreateSessionSpec { title: Some("shell".to_string()), launch: Some(cat_launch_spec()) },
        )
        .await
        .expect("create_session should succeed");
    let topology = fixture
        .client
        .topology_snapshot(created.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let pane_id = topology.tabs[0].focused_pane.expect("focused pane should exist");
    wait_for_screen_line(&fixture, created.session.session_id, pane_id, "ready").await;
    let mut subscription = fixture
        .client
        .open_subscription(created.session.session_id, SubscriptionSpec::PaneSurface { pane_id })
        .await
        .expect("subscription should open");

    let initial = must_recv_subscription_event(&mut subscription).await;
    let initial = match initial {
        SubscriptionEvent::ScreenDelta(delta) => delta,
        other => panic!("unexpected initial event: {other:?}"),
    };
    let dispatch = fixture
        .client
        .dispatch(
            created.session.session_id,
            MuxCommand::SendInput(SendInputSpec {
                pane_id,
                data: submitted_input("hello from pane stream"),
                client_event_id: None,
            }),
        )
        .await
        .expect("dispatch should succeed");
    let updated = must_recv_subscription_event(&mut subscription).await;
    let updated = match updated {
        SubscriptionEvent::ScreenDelta(delta) => delta,
        other => panic!("unexpected screen event: {other:?}"),
    };
    let patch = updated.patch.expect("delta patch should exist");

    assert!(!dispatch.changed);
    assert!(initial.full_replace.is_some());
    assert_ne!(updated.to_sequence, updated.from_sequence);
    assert!(
        patch.line_updates.iter().any(|line| line.line.text.contains("hello from pane stream"))
    );
    assert!(updated.full_replace.is_none());

    fixture.shutdown().await.expect("fixture should stop cleanly");
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_smoke_roundtrips_live_pty_io() {
    let fixture = daemon_fixture("bootstrap-pty-smoke").expect("fixture should start");
    let created = fixture
        .client
        .create_session(
            BackendKind::Native,
            CreateSessionSpec { title: Some("shell".to_string()), launch: Some(cat_launch_spec()) },
        )
        .await
        .expect("create_session should succeed");
    let topology = fixture
        .client
        .topology_snapshot(created.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let pane_id = topology.tabs[0].focused_pane.expect("focused pane should exist");

    wait_for_screen_line(&fixture, created.session.session_id, pane_id, "ready").await;
    let before = fixture
        .client
        .screen_snapshot(created.session.session_id, pane_id)
        .await
        .expect("screen_snapshot should succeed");
    let dispatch = fixture
        .client
        .dispatch(
            created.session.session_id,
            MuxCommand::SendInput(SendInputSpec {
                pane_id,
                data: submitted_input("hello from smoke"),
                client_event_id: None,
            }),
        )
        .await
        .expect("dispatch should succeed");

    assert!(!dispatch.changed);
    wait_for_screen_line(&fixture, created.session.session_id, pane_id, "hello from smoke").await;
    let delta = fixture
        .client
        .screen_delta(created.session.session_id, pane_id, before.sequence)
        .await
        .expect("screen_delta should succeed");
    let patch = delta.patch.expect("delta patch should exist");

    assert_eq!(delta.pane_id, pane_id);
    assert_eq!(delta.from_sequence, before.sequence);
    assert!(delta.to_sequence > before.sequence);
    assert!(patch.line_updates.iter().any(|line| line.line.text.contains("hello from smoke")));
    assert!(delta.full_replace.is_none());

    fixture.shutdown().await.expect("fixture should stop cleanly");
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_smoke_streams_surface_updates_for_all_native_panes_after_resize() {
    let fixture = daemon_fixture("bootstrap-native-pane-resize-sub").expect("fixture should start");
    let created = fixture
        .client
        .create_session(
            BackendKind::Native,
            CreateSessionSpec { title: Some("shell".to_string()), launch: Some(cat_launch_spec()) },
        )
        .await
        .expect("create_session should succeed");
    let topology = fixture
        .client
        .topology_snapshot(created.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let original_pane = topology.tabs[0].focused_pane.expect("focused pane should exist");

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
    let resized_pane = pane_ids
        .iter()
        .copied()
        .find(|candidate| *candidate != original_pane)
        .expect("new pane should exist");
    wait_for_screen_line(&fixture, created.session.session_id, resized_pane, "ready").await;

    let mut original_subscription = fixture
        .client
        .open_subscription(
            created.session.session_id,
            SubscriptionSpec::PaneSurface { pane_id: original_pane },
        )
        .await
        .expect("original subscription should open");
    let mut resized_subscription = fixture
        .client
        .open_subscription(
            created.session.session_id,
            SubscriptionSpec::PaneSurface { pane_id: resized_pane },
        )
        .await
        .expect("resized subscription should open");

    let original_initial = match must_recv_subscription_event(&mut original_subscription).await {
        SubscriptionEvent::ScreenDelta(delta) => delta,
        other => panic!("unexpected original initial event: {other:?}"),
    };
    let resized_initial = match must_recv_subscription_event(&mut resized_subscription).await {
        SubscriptionEvent::ScreenDelta(delta) => delta,
        other => panic!("unexpected resized initial event: {other:?}"),
    };

    let original_before = fixture
        .client
        .screen_snapshot(created.session.session_id, original_pane)
        .await
        .expect("screen_snapshot should succeed");
    let resized_before = fixture
        .client
        .screen_snapshot(created.session.session_id, resized_pane)
        .await
        .expect("screen_snapshot should succeed");
    let total_cols = original_before.cols.saturating_add(resized_before.cols);
    let target_cols = resized_before.cols.saturating_add(10).min(total_cols.saturating_sub(1));
    let resize = fixture
        .client
        .dispatch(
            created.session.session_id,
            MuxCommand::ResizePane(ResizePaneSpec {
                pane_id: resized_pane,
                rows: resized_before.rows,
                cols: target_cols,
            }),
        )
        .await
        .expect("resize should succeed");

    let original_updated = match must_recv_subscription_event(&mut original_subscription).await {
        SubscriptionEvent::ScreenDelta(delta) => delta,
        other => panic!("unexpected original updated event: {other:?}"),
    };
    let resized_updated = match must_recv_subscription_event(&mut resized_subscription).await {
        SubscriptionEvent::ScreenDelta(delta) => delta,
        other => panic!("unexpected resized updated event: {other:?}"),
    };

    assert!(original_initial.full_replace.is_some());
    assert!(resized_initial.full_replace.is_some());
    assert!(resize.changed);
    assert_eq!(original_updated.pane_id, original_pane);
    assert_eq!(resized_updated.pane_id, resized_pane);
    assert!(original_updated.full_replace.is_some());
    assert!(resized_updated.full_replace.is_some());

    fixture.shutdown().await.expect("fixture should stop cleanly");
}
