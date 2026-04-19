use std::{thread, time::Duration};

use terminal_backend_api::{
    CreateSessionSpec, MuxCommand, NewTabSpec, SendInputSpec, ShellLaunchSpec, SubscriptionSpec,
};
use terminal_domain::BackendKind;
use terminal_protocol::SubscriptionEvent;
use terminal_testing::{daemon_fixture, daemon_state};

#[test]
fn bootstrap_smoke_exposes_empty_daemon_state() {
    let daemon = daemon_state();
    let handshake = daemon.handshake();

    assert_eq!(handshake.protocol_version.major, 0);
    assert_eq!(handshake.protocol_version.minor, 1);
    assert_eq!(handshake.available_backends.len(), 3);
    assert_eq!(daemon.session_count(), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_smoke_roundtrips_request_reply_flow() {
    let fixture = daemon_fixture("bootstrap-smoke").expect("fixture should start");
    let created = fixture
        .client
        .create_session(
            BackendKind::Native,
            CreateSessionSpec { title: Some("shell".to_string()), ..CreateSessionSpec::default() },
        )
        .await
        .expect("create_session should succeed");
    let handshake = fixture.client.handshake().await.expect("handshake should succeed");
    let sessions = fixture.client.list_sessions().await.expect("list_sessions should succeed");
    let topology = fixture
        .client
        .topology_snapshot(created.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let pane_id = topology.tabs[0].focused_pane.expect("focused pane should exist");
    let screen = fixture
        .client
        .screen_snapshot(created.session.session_id, pane_id)
        .await
        .expect("screen_snapshot should succeed");
    let delta = fixture
        .client
        .screen_delta(created.session.session_id, pane_id, screen.sequence)
        .await
        .expect("screen_delta should succeed");
    let dispatch = fixture
        .client
        .dispatch(
            created.session.session_id,
            MuxCommand::NewTab(NewTabSpec { title: Some("logs".to_string()) }),
        )
        .await
        .expect("dispatch should succeed");
    let topology_after_dispatch = fixture
        .client
        .topology_snapshot(created.session.session_id)
        .await
        .expect("topology_snapshot should succeed");

    assert_eq!(handshake.protocol_version.major, 0);
    assert_eq!(handshake.protocol_version.minor, 1);
    assert_eq!(created.session.route.backend, BackendKind::Native);
    assert_eq!(created.session.title.as_deref(), Some("shell"));
    assert_eq!(sessions.sessions.len(), 1);
    assert_eq!(sessions.sessions[0].session_id, created.session.session_id);
    assert_eq!(topology.session_id, created.session.session_id);
    assert_eq!(screen.pane_id, pane_id);
    assert!(!screen.surface.lines.is_empty());
    assert_eq!(delta.rows, screen.rows);
    assert_eq!(delta.cols, screen.cols);
    assert!(delta.patch.is_none());
    assert_eq!(delta.from_sequence, screen.sequence);
    assert_eq!(delta.to_sequence, screen.sequence);
    assert!(delta.full_replace.is_none());
    assert!(dispatch.changed);
    assert_eq!(topology_after_dispatch.tabs.len(), 2);

    fixture.shutdown().await.expect("fixture should stop cleanly");
}

#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_smoke_streams_topology_updates() {
    let fixture = daemon_fixture("bootstrap-subscription-smoke").expect("fixture should start");
    let created = fixture
        .client
        .create_session(
            BackendKind::Native,
            CreateSessionSpec { title: Some("shell".to_string()), ..CreateSessionSpec::default() },
        )
        .await
        .expect("create_session should succeed");
    let mut subscription = fixture
        .client
        .open_subscription(created.session.session_id, SubscriptionSpec::SessionTopology)
        .await
        .expect("subscription should open");

    let initial = subscription.recv().await.expect("recv should succeed").expect("event");
    let initial = match initial {
        SubscriptionEvent::TopologySnapshot(snapshot) => snapshot,
        other => panic!("unexpected initial event: {other:?}"),
    };
    let dispatch = fixture
        .client
        .dispatch(
            created.session.session_id,
            MuxCommand::NewTab(NewTabSpec { title: Some("logs".to_string()) }),
        )
        .await
        .expect("dispatch should succeed");
    let updated = subscription.recv().await.expect("recv should succeed").expect("event");
    let updated = match updated {
        SubscriptionEvent::TopologySnapshot(snapshot) => snapshot,
        other => panic!("unexpected topology event: {other:?}"),
    };

    assert_eq!(initial.tabs.len(), 1);
    assert!(dispatch.changed);
    assert_eq!(updated.tabs.len(), 2);

    fixture.shutdown().await.expect("fixture should stop cleanly");
}

#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_smoke_closes_subscription_lane_explicitly() {
    let fixture = daemon_fixture("bootstrap-sub-close").expect("fixture should start");
    let created = fixture
        .client
        .create_session(
            BackendKind::Native,
            CreateSessionSpec { title: Some("shell".to_string()), ..CreateSessionSpec::default() },
        )
        .await
        .expect("create_session should succeed");
    let mut subscription = fixture
        .client
        .open_subscription(created.session.session_id, SubscriptionSpec::SessionTopology)
        .await
        .expect("subscription should open");

    let initial = subscription.recv().await.expect("recv should succeed").expect("event");
    match initial {
        SubscriptionEvent::TopologySnapshot(_) => {}
        other => panic!("unexpected initial event: {other:?}"),
    }
    subscription.close().await.expect("close should succeed");
    assert!(subscription.recv().await.expect("recv should succeed").is_none());

    fixture.shutdown().await.expect("fixture should stop cleanly");
}

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

    let initial = subscription.recv().await.expect("recv should succeed").expect("event");
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
                data: "hello from pane stream\r".to_string(),
            }),
        )
        .await
        .expect("dispatch should succeed");
    let updated = subscription.recv().await.expect("recv should succeed").expect("event");
    let updated = match updated {
        SubscriptionEvent::ScreenDelta(delta) => delta,
        other => panic!("unexpected screen event: {other:?}"),
    };
    let patch = updated.patch.expect("delta patch should exist");

    assert!(!dispatch.changed);
    assert!(initial.full_replace.is_some());
    assert!(updated.to_sequence > updated.from_sequence);
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
                data: "hello from smoke\r".to_string(),
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
fn cat_launch_spec() -> ShellLaunchSpec {
    ShellLaunchSpec::new("/bin/sh").with_args(["-lc", "printf 'ready\\n'; exec cat"])
}

#[cfg(unix)]
async fn wait_for_screen_line(
    fixture: &terminal_testing::DaemonFixture,
    session_id: terminal_domain::SessionId,
    pane_id: terminal_domain::PaneId,
    needle: &str,
) {
    for _ in 0..40 {
        let screen = fixture
            .client
            .screen_snapshot(session_id, pane_id)
            .await
            .expect("screen_snapshot should succeed");
        if screen.surface.lines.iter().any(|line| line.text.contains(needle)) {
            return;
        }
        thread::sleep(Duration::from_millis(50));
    }

    panic!("screen never contained expected text: {needle}");
}
