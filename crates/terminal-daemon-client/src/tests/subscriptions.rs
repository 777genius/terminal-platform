use super::{prelude::*, support::*};

#[tokio::test(flavor = "multi_thread")]
async fn streams_topology_updates_over_subscription_lane() {
    let address = unique_address("daemon-client-sub-topology");
    let server =
        spawn_local_socket_server(isolated_daemon(), address.clone()).expect("server should bind");
    let client = LocalSocketDaemonClient::new(address);
    let created = client
        .create_session(
            BackendKind::Native,
            CreateSessionSpec { title: Some("shell".to_string()), ..CreateSessionSpec::default() },
        )
        .await
        .expect("create_session should succeed");
    let mut subscription = client
        .open_subscription(created.session.session_id, SubscriptionSpec::SessionTopology)
        .await
        .expect("subscription should open");

    let initial = must_recv_subscription_event(&mut subscription).await;
    let initial = match initial {
        SubscriptionEvent::TopologySnapshot(snapshot) => snapshot,
        other => panic!("unexpected initial event: {other:?}"),
    };
    let result = client
        .dispatch(
            created.session.session_id,
            MuxCommand::NewTab(NewTabSpec { title: Some("logs".to_string()) }),
        )
        .await
        .expect("dispatch should succeed");
    let updated = must_recv_subscription_event(&mut subscription).await;
    let updated = match updated {
        SubscriptionEvent::TopologySnapshot(snapshot) => snapshot,
        other => panic!("unexpected topology event: {other:?}"),
    };

    assert_eq!(initial.tabs.len(), 1);
    assert!(result.changed);
    assert_eq!(updated.tabs.len(), 2);

    server.shutdown().await.expect("server shutdown should succeed");
}

#[tokio::test(flavor = "multi_thread")]
async fn closes_topology_subscription_lane_explicitly() {
    let address = unique_address("daemon-client-sub-close");
    let server =
        spawn_local_socket_server(isolated_daemon(), address.clone()).expect("server should bind");
    let client = LocalSocketDaemonClient::new(address);
    let created = client
        .create_session(
            BackendKind::Native,
            CreateSessionSpec { title: Some("shell".to_string()), ..CreateSessionSpec::default() },
        )
        .await
        .expect("create_session should succeed");
    let mut subscription = client
        .open_subscription(created.session.session_id, SubscriptionSpec::SessionTopology)
        .await
        .expect("subscription should open");

    let initial = must_recv_subscription_event(&mut subscription).await;
    match initial {
        SubscriptionEvent::TopologySnapshot(_) => {}
        other => panic!("unexpected initial event: {other:?}"),
    }
    subscription.close().await.expect("close should succeed");
    assert!(recv_subscription_event(&mut subscription).await.is_none());

    server.shutdown().await.expect("server shutdown should succeed");
}

#[tokio::test(flavor = "multi_thread")]
async fn closes_topology_subscription_lane_with_buffered_events() {
    let address = unique_address("daemon-client-sub-close-backlog");
    let server =
        spawn_local_socket_server(isolated_daemon(), address.clone()).expect("server should bind");
    let client = LocalSocketDaemonClient::new(address);
    let created = client
        .create_session(
            BackendKind::Native,
            CreateSessionSpec { title: Some("shell".to_string()), ..CreateSessionSpec::default() },
        )
        .await
        .expect("create_session should succeed");
    let topology = client
        .topology_snapshot(created.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let tab_id = topology.focused_tab.expect("focused tab should exist");
    let mut subscription = client
        .open_subscription(created.session.session_id, SubscriptionSpec::SessionTopology)
        .await
        .expect("subscription should open");

    let initial = must_recv_subscription_event(&mut subscription).await;
    assert!(matches!(initial, SubscriptionEvent::TopologySnapshot(_)));

    for revision in 0..24 {
        client
            .dispatch(
                created.session.session_id,
                MuxCommand::RenameTab { tab_id, title: format!("close-backlog-{revision}") },
            )
            .await
            .expect("rename tab should succeed");
    }

    subscription.close().await.expect("close should succeed");
    assert!(recv_subscription_event(&mut subscription).await.is_none());

    server.shutdown().await.expect("server shutdown should succeed");
}

#[tokio::test(flavor = "multi_thread")]
async fn closes_topology_subscription_lane_when_server_shuts_down() {
    let address = unique_address("daemon-client-sub-server-shutdown");
    let server =
        spawn_local_socket_server(isolated_daemon(), address.clone()).expect("server should bind");
    let client = LocalSocketDaemonClient::new(address);
    let created = client
        .create_session(
            BackendKind::Native,
            CreateSessionSpec { title: Some("shell".to_string()), ..CreateSessionSpec::default() },
        )
        .await
        .expect("create_session should succeed");
    let mut subscription = client
        .open_subscription(created.session.session_id, SubscriptionSpec::SessionTopology)
        .await
        .expect("subscription should open");

    let initial = must_recv_subscription_event(&mut subscription).await;
    match initial {
        SubscriptionEvent::TopologySnapshot(_) => {}
        other => panic!("unexpected initial event: {other:?}"),
    }

    server.shutdown().await.expect("server shutdown should succeed");

    let closed = timeout(Duration::from_secs(3), subscription.recv())
        .await
        .expect("subscription should close after server shutdown")
        .expect("recv should succeed");
    assert!(closed.is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn closes_topology_subscription_lane_cleanly_after_server_shutdown() {
    let address = unique_address("sub-close-down");
    let server =
        spawn_local_socket_server(isolated_daemon(), address.clone()).expect("server should bind");
    let client = LocalSocketDaemonClient::new(address);
    let created = client
        .create_session(
            BackendKind::Native,
            CreateSessionSpec { title: Some("shell".to_string()), ..CreateSessionSpec::default() },
        )
        .await
        .expect("create_session should succeed");
    let mut subscription = client
        .open_subscription(created.session.session_id, SubscriptionSpec::SessionTopology)
        .await
        .expect("subscription should open");

    let initial = must_recv_subscription_event(&mut subscription).await;
    assert!(matches!(initial, SubscriptionEvent::TopologySnapshot(_)));

    server.shutdown().await.expect("server shutdown should succeed");
    subscription.close().await.expect("close should tolerate a shutdown transport disconnect");
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn streams_live_pane_surface_updates_over_subscription_lane() {
    let address = unique_address("daemon-client-sub-pane");
    let server =
        spawn_local_socket_server(isolated_daemon(), address.clone()).expect("server should bind");
    let client = LocalSocketDaemonClient::new(address);
    let created = client
        .create_session(
            BackendKind::Native,
            CreateSessionSpec { title: Some("shell".to_string()), launch: Some(cat_launch_spec()) },
        )
        .await
        .expect("create_session should succeed");
    let topology = client
        .topology_snapshot(created.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let pane_id = topology.tabs[0].focused_pane.expect("focused pane should exist");
    wait_for_screen_line(&client, created.session.session_id, pane_id, "ready").await;
    let mut subscription = client
        .open_subscription(created.session.session_id, SubscriptionSpec::PaneSurface { pane_id })
        .await
        .expect("subscription should open");

    let initial = must_recv_subscription_event(&mut subscription).await;
    let initial = match initial {
        SubscriptionEvent::ScreenDelta(delta) => delta,
        other => panic!("unexpected initial event: {other:?}"),
    };
    let result = client
        .dispatch(
            created.session.session_id,
            MuxCommand::SendInput(SendInputSpec {
                pane_id,
                data: submitted_input("hello from subscription"),
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

    assert!(!result.changed);
    assert!(initial.full_replace.is_some());
    assert!(updated.to_sequence > updated.from_sequence);
    assert!(
        patch.line_updates.iter().any(|line| line.line.text.contains("hello from subscription"))
    );
    assert!(updated.full_replace.is_none());

    server.shutdown().await.expect("server shutdown should succeed");
}
