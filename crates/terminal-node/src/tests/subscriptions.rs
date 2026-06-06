use super::{prelude::*, support::*};

#[tokio::test(flavor = "multi_thread")]
async fn streams_subscription_events_through_node_surface() {
    let fixture = daemon_fixture("terminal-node-subscriptions").expect("fixture should start");
    let node = NodeHostClient::new(fixture.client.address().clone());
    let created = node
        .create_native_session(&cat_launch_request("shell"))
        .await
        .expect("create_native_session should succeed");
    let attached =
        node.attach_session(&created.session_id).await.expect("attach_session should succeed");
    let pane_id =
        attached.focused_screen.as_ref().expect("focused screen should exist").pane_id.clone();
    wait_for_interactive_screen(&node, &created.session_id, &pane_id, "node-host-subscriptions")
        .await;

    let topology_subscription = node
        .open_subscription(&created.session_id, &NodeSubscriptionSpec::SessionTopology)
        .await
        .expect("topology subscription should open");
    let initial_topology = topology_subscription
        .next_event()
        .await
        .expect("initial topology event should arrive")
        .expect("initial topology event should exist");

    let pane_subscription = node
        .open_subscription(
            &created.session_id,
            &NodeSubscriptionSpec::PaneSurface { pane_id: pane_id.clone() },
        )
        .await
        .expect("pane subscription should open");
    let initial_pane = pane_subscription
        .next_event()
        .await
        .expect("initial pane event should arrive")
        .expect("initial pane event should exist");

    node.dispatch_mux_command(
        &created.session_id,
        &NodeMuxCommand::NewTab(NodeNewTabCommand { title: Some("logs".to_string()) }),
    )
    .await
    .expect("new tab should succeed");
    let topology_update = next_topology_snapshot(&topology_subscription)
        .await
        .expect("topology snapshot should arrive");

    node.dispatch_mux_command(
        &created.session_id,
        &NodeMuxCommand::SendInput(NodeSendInputCommand {
            pane_id: pane_id.clone(),
            data: submitted_input("node subscription input"),
            client_event_id: None,
        }),
    )
    .await
    .expect("send input should succeed");
    let pane_update = wait_for_subscription_line(&pane_subscription, "node subscription input")
        .await
        .expect("pane update should arrive");

    assert_eq!(
        initial_topology,
        NodeSubscriptionEvent::TopologySnapshot(attached.topology.clone())
    );
    assert!(matches!(
        initial_pane,
        NodeSubscriptionEvent::ScreenDelta(delta) if delta.full_replace.is_some()
    ));
    assert_eq!(topology_update.tabs.len(), 2);
    assert!(subscription_delta_contains(&pane_update, "node subscription input"));

    topology_subscription.close().await;
    pane_subscription.close().await;
    fixture.shutdown().await.expect("fixture should stop cleanly");
}

#[tokio::test(flavor = "multi_thread")]
async fn closes_subscription_stream_when_daemon_shuts_down() {
    let fixture = daemon_fixture("terminal-node-close").expect("fixture should start");
    let node = NodeHostClient::new(fixture.client.address().clone());
    let created = node
        .create_native_session(&cat_launch_request("shutdown"))
        .await
        .expect("create_native_session should succeed");
    let attached =
        node.attach_session(&created.session_id).await.expect("attach_session should succeed");
    let pane_id =
        attached.focused_screen.as_ref().expect("focused screen should exist").pane_id.clone();
    let pane_subscription = node
        .open_subscription(
            &created.session_id,
            &NodeSubscriptionSpec::PaneSurface { pane_id: pane_id.clone() },
        )
        .await
        .expect("pane subscription should open");

    let initial = pane_subscription
        .next_event()
        .await
        .expect("initial pane event should arrive")
        .expect("initial pane event should exist");
    assert!(matches!(
        initial,
        NodeSubscriptionEvent::ScreenDelta(delta) if delta.full_replace.is_some()
    ));

    fixture.shutdown().await.expect("fixture should stop cleanly");

    assert!(wait_for_subscription_close(&pane_subscription).await);
}

#[tokio::test(flavor = "multi_thread")]
async fn closes_subscription_bridge_under_backpressure() {
    let fixture = daemon_fixture("terminal-node-backpressure").expect("fixture should start");
    let node = NodeHostClient::new(fixture.client.address().clone());
    let created = node
        .create_native_session(&cat_launch_request("backpressure"))
        .await
        .expect("create_native_session should succeed");
    let topology = node
        .topology_snapshot(&created.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let tab_id = topology.focused_tab.clone().expect("focused tab should exist");
    let subscription = node
        .open_subscription(&created.session_id, &NodeSubscriptionSpec::SessionTopology)
        .await
        .expect("topology subscription should open");

    let initial = subscription
        .next_event()
        .await
        .expect("initial topology event should arrive")
        .expect("initial topology event should exist");
    assert!(matches!(initial, NodeSubscriptionEvent::TopologySnapshot(_)));

    for revision in 0..96 {
        node.dispatch_mux_command(
            &created.session_id,
            &NodeMuxCommand::RenameTab(NodeRenameTabCommand {
                tab_id: tab_id.clone(),
                title: format!("backpressure-{revision}"),
            }),
        )
        .await
        .expect("rename tab should succeed");
    }

    timeout(operation_timeout(), subscription.close())
        .await
        .expect("subscription close should not hang under backpressure");
    timeout(operation_timeout(), fixture.shutdown())
        .await
        .expect("fixture shutdown should not hang after backpressure close")
        .expect("fixture should stop cleanly");
}

#[tokio::test(flavor = "multi_thread")]
async fn repeatedly_reopens_subscriptions_through_node_surface() {
    let fixture = daemon_fixture("terminal-node-reopen").expect("fixture should start");
    let node = NodeHostClient::new(fixture.client.address().clone());
    let created = node
        .create_native_session(&cat_launch_request("reopen"))
        .await
        .expect("create_native_session should succeed");
    let attached =
        node.attach_session(&created.session_id).await.expect("attach_session should succeed");
    let pane_id =
        attached.focused_screen.as_ref().expect("focused screen should exist").pane_id.clone();
    wait_for_interactive_screen(&node, &created.session_id, &pane_id, "node-host-reopen").await;

    for cycle in 0..24 {
        let topology_subscription = timeout(
            operation_timeout(),
            node.open_subscription(&created.session_id, &NodeSubscriptionSpec::SessionTopology),
        )
        .await
        .expect("topology subscription open should not hang")
        .expect("topology subscription should open");
        let initial_topology = timeout(operation_timeout(), topology_subscription.next_event())
            .await
            .expect("topology subscription next_event should not hang")
            .expect("topology subscription should stay healthy")
            .expect("topology subscription should yield initial event");
        assert!(
            matches!(
                initial_topology,
                NodeSubscriptionEvent::TopologySnapshot(snapshot)
                    if snapshot.session_id == created.session_id
            ),
            "cycle {cycle} should receive an initial topology snapshot"
        );
        timeout(operation_timeout(), topology_subscription.close())
            .await
            .expect("topology subscription close should not hang");

        let pane_subscription = timeout(
            operation_timeout(),
            node.open_subscription(
                &created.session_id,
                &NodeSubscriptionSpec::PaneSurface { pane_id: pane_id.clone() },
            ),
        )
        .await
        .expect("pane subscription open should not hang")
        .expect("pane subscription should open");
        let initial_pane = timeout(operation_timeout(), pane_subscription.next_event())
            .await
            .expect("pane subscription next_event should not hang")
            .expect("pane subscription should stay healthy")
            .expect("pane subscription should yield initial event");
        assert!(
            matches!(
                initial_pane,
                NodeSubscriptionEvent::ScreenDelta(delta) if delta.full_replace.is_some()
            ),
            "cycle {cycle} should receive an initial pane delta"
        );

        if cycle % 6 == 5 {
            let marker = format!("node reopen cycle {cycle}");
            node.dispatch_mux_command(
                &created.session_id,
                &NodeMuxCommand::SendInput(NodeSendInputCommand {
                    pane_id: pane_id.clone(),
                    data: submitted_input(&marker),
                    client_event_id: None,
                }),
            )
            .await
            .expect("send input should succeed during reopen stress");
            let update = timeout(
                operation_timeout(),
                wait_for_subscription_line(&pane_subscription, &marker),
            )
            .await
            .expect("pane update wait should not hang")
            .expect("pane update should arrive");
            assert!(
                subscription_delta_contains(&update, &marker),
                "cycle {cycle} should receive the live pane update"
            );
        }

        timeout(operation_timeout(), pane_subscription.close())
            .await
            .expect("pane subscription close should not hang");
    }

    let final_list = node.list_sessions().await.expect("list_sessions should succeed");
    assert!(final_list.iter().any(|session| session.session_id == created.session_id));

    fixture.shutdown().await.expect("fixture should stop cleanly");
}
