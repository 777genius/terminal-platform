use super::{prelude::*, support::*};

#[tokio::test(flavor = "multi_thread")]
async fn restarts_server_on_same_address_across_multiple_cycles() {
    let address = unique_address("daemon-client-restart-cycles");
    let client = LocalSocketDaemonClient::new(address.clone());

    for cycle in 0..3 {
        let server = spawn_default_daemon_with_retry(address.clone()).expect("server should bind");

        let handshake = client.handshake().await.expect("handshake should succeed");
        assert_eq!(handshake.daemon_phase, DaemonPhase::Ready, "cycle {cycle} should be ready");

        server.shutdown().await.expect("server shutdown should succeed");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn repeatedly_opens_and_closes_topology_subscriptions() {
    let address = unique_address("daemon-client-subscribe-cycles");
    let server = spawn_local_socket_server(TerminalDaemon::default(), address.clone())
        .expect("server should bind");
    let client = LocalSocketDaemonClient::new(address);
    let created = client
        .create_session(
            BackendKind::Native,
            CreateSessionSpec {
                title: Some("subscribe-cycles".to_string()),
                ..CreateSessionSpec::default()
            },
        )
        .await
        .expect("create_session should succeed");

    for cycle in 0..24 {
        let mut subscription = client
            .open_subscription(created.session.session_id, SubscriptionSpec::SessionTopology)
            .await
            .expect("subscription should open");
        let initial = recv_subscription_event(&mut subscription).await;

        assert!(
            matches!(initial, Some(SubscriptionEvent::TopologySnapshot(_))),
            "cycle {cycle} should receive initial topology snapshot"
        );

        subscription.close().await.expect("subscription should close cleanly");
    }

    server.shutdown().await.expect("server shutdown should succeed");
}
