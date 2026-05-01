use super::{prelude::*, support::*};

#[tokio::test(flavor = "multi_thread")]
async fn recovers_node_host_client_after_daemon_restart() {
    let address = unique_socket_address("terminal-node-restart");
    let readiness_client = LocalSocketDaemonClient::new(address.clone());
    let node = NodeHostClient::new(address.clone());
    let server = spawn_daemon_with_retry(address.clone()).expect("initial daemon should bind");
    wait_for_daemon_ready(&readiness_client).await;

    let initial_list = timeout(operation_timeout(), node.list_sessions())
        .await
        .expect("initial list_sessions should not hang")
        .expect("initial list_sessions should succeed");
    assert!(initial_list.is_empty());

    server.shutdown().await.expect("initial daemon should stop cleanly");

    let stale_result = timeout(operation_timeout(), node.list_sessions())
        .await
        .expect("stale list_sessions should not hang");
    assert!(stale_result.is_err(), "stale daemon request should fail");

    let restarted_readiness_client = LocalSocketDaemonClient::new(address.clone());
    let replacement =
        spawn_daemon_with_retry(address.clone()).expect("replacement daemon should bind");
    wait_for_daemon_ready(&restarted_readiness_client).await;

    let created =
        timeout(operation_timeout(), node.create_native_session(&cat_launch_request("restart")))
            .await
            .expect("post-restart create_native_session should not hang")
            .expect("post-restart create_native_session should succeed");
    let attached = timeout(operation_timeout(), node.attach_session(&created.session_id))
        .await
        .expect("post-restart attach_session should not hang")
        .expect("post-restart attach_session should succeed");
    let pane_id = attached
        .focused_screen
        .as_ref()
        .expect("focused screen should exist after restart")
        .pane_id
        .clone();
    let subscription = timeout(
        operation_timeout(),
        node.open_subscription(
            &created.session_id,
            &NodeSubscriptionSpec::PaneSurface { pane_id: pane_id.clone() },
        ),
    )
    .await
    .expect("post-restart subscription open should not hang")
    .expect("post-restart subscription should open");
    let initial_event = timeout(operation_timeout(), subscription.next_event())
        .await
        .expect("post-restart subscription next_event should not hang")
        .expect("post-restart subscription should stay healthy")
        .expect("post-restart subscription should yield an event");

    assert!(matches!(
        initial_event,
        NodeSubscriptionEvent::ScreenDelta(delta) if delta.full_replace.is_some()
    ));

    timeout(operation_timeout(), subscription.close())
        .await
        .expect("post-restart subscription close should not hang");
    replacement.shutdown().await.expect("replacement daemon should stop cleanly");
}

#[tokio::test(flavor = "multi_thread")]
async fn recovers_node_host_client_across_multiple_daemon_restart_cycles() {
    let address = unique_socket_address("terminal-node-restart-cycles");
    let node = NodeHostClient::new(address.clone());

    for cycle in 0..3 {
        let readiness_client = LocalSocketDaemonClient::new(address.clone());
        let server =
            spawn_daemon_with_retry(address.clone()).expect("daemon should bind for restart cycle");
        wait_for_daemon_ready(&readiness_client).await;

        let listed = timeout(operation_timeout(), node.list_sessions())
            .await
            .expect("list_sessions should not hang")
            .expect("list_sessions should succeed");
        assert!(listed.is_empty(), "cycle {cycle} should start with a fresh daemon state");

        let created = timeout(
            operation_timeout(),
            node.create_native_session(&cat_launch_request(&format!("restart-cycle-{cycle}"))),
        )
        .await
        .expect("create_native_session should not hang")
        .expect("create_native_session should succeed");
        let attached = timeout(operation_timeout(), node.attach_session(&created.session_id))
            .await
            .expect("attach_session should not hang")
            .expect("attach_session should succeed");
        let pane_id = attached
            .focused_screen
            .as_ref()
            .expect("focused screen should exist after restart cycle")
            .pane_id
            .clone();
        let subscription = timeout(
            operation_timeout(),
            node.open_subscription(
                &created.session_id,
                &NodeSubscriptionSpec::PaneSurface { pane_id: pane_id.clone() },
            ),
        )
        .await
        .expect("subscription open should not hang")
        .expect("subscription should open");
        let initial_event = timeout(operation_timeout(), subscription.next_event())
            .await
            .expect("subscription next_event should not hang")
            .expect("subscription should stay healthy")
            .expect("subscription should yield an event");
        assert!(
            matches!(
                initial_event,
                NodeSubscriptionEvent::ScreenDelta(delta) if delta.full_replace.is_some()
            ),
            "cycle {cycle} should receive an initial pane delta"
        );
        timeout(operation_timeout(), subscription.close())
            .await
            .expect("subscription close should not hang");

        timeout(operation_timeout(), server.shutdown())
            .await
            .expect("daemon shutdown should not hang")
            .expect("daemon should stop cleanly");

        let stale_result = timeout(operation_timeout(), node.list_sessions())
            .await
            .expect("stale list_sessions should not hang");
        assert!(stale_result.is_err(), "cycle {cycle} stale request should fail");
    }
}
