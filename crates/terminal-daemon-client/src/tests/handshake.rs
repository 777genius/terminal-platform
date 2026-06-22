use super::{prelude::*, support::*};

#[tokio::test(flavor = "multi_thread")]
async fn roundtrips_handshake_and_empty_list_sessions() {
    let address = unique_address("daemon-client");
    let server =
        spawn_local_socket_server(isolated_daemon(), address.clone()).expect("server should bind");
    let client = LocalSocketDaemonClient::new(address);

    let handshake = client.handshake().await.expect("handshake should succeed");
    let assessment =
        client.handshake_assessment().await.expect("handshake_assessment should succeed");
    let sessions = client.list_sessions().await.expect("list_sessions should succeed");

    assert_eq!(handshake.protocol_version.major, 0);
    assert_eq!(handshake.protocol_version.minor, 2);
    assert_eq!(handshake.daemon_phase, DaemonPhase::Ready);
    assert!(handshake.capabilities.request_reply);
    assert!(handshake.capabilities.topology_subscriptions);
    assert!(handshake.capabilities.pane_subscriptions);
    assert!(handshake.capabilities.backend_discovery);
    assert!(handshake.capabilities.backend_capability_queries);
    assert!(handshake.capabilities.saved_sessions);
    assert!(handshake.capabilities.session_restore);
    assert!(handshake.capabilities.degraded_error_reasons);
    assert!(handshake.capabilities.session_health);
    assert_eq!(client.info().expected_protocol, handshake.protocol_version);
    assert!(assessment.can_use);
    assert_eq!(assessment.status, HandshakeAssessmentStatus::Ready);
    assert!(sessions.sessions.is_empty());

    server.shutdown().await.expect("server shutdown should succeed");
}

#[test]
fn assesses_handshake_protocol_and_phase() {
    let info = DaemonClientInfo::default();
    let starting = info.assess_handshake(&Handshake {
        protocol_version: ProtocolVersion { major: 0, minor: 2 },
        binary_version: CURRENT_BINARY_VERSION.to_string(),
        daemon_phase: DaemonPhase::Starting,
        capabilities: DaemonCapabilities {
            request_reply: true,
            topology_subscriptions: true,
            pane_subscriptions: true,
            backend_discovery: true,
            backend_capability_queries: true,
            saved_sessions: true,
            session_restore: true,
            degraded_error_reasons: true,
            session_health: true,
        },
        available_backends: vec![BackendKind::Native],
        session_scope: "current_user".to_string(),
    });
    let incompatible = info.assess_handshake(&Handshake {
        protocol_version: ProtocolVersion { major: 0, minor: 3 },
        binary_version: CURRENT_BINARY_VERSION.to_string(),
        daemon_phase: DaemonPhase::Ready,
        capabilities: DaemonCapabilities {
            request_reply: true,
            topology_subscriptions: true,
            pane_subscriptions: true,
            backend_discovery: true,
            backend_capability_queries: true,
            saved_sessions: true,
            session_restore: true,
            degraded_error_reasons: true,
            session_health: true,
        },
        available_backends: vec![BackendKind::Native],
        session_scope: "current_user".to_string(),
    });

    assert!(!starting.can_use);
    assert_eq!(starting.status, HandshakeAssessmentStatus::Starting);
    assert!(starting.protocol.can_connect);
    assert!(!incompatible.can_use);
    assert_eq!(incompatible.status, HandshakeAssessmentStatus::ProtocolMinorAhead);
    assert!(!incompatible.protocol.can_connect);
}
