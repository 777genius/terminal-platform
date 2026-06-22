use super::{prelude::*, support::*};

#[tokio::test(flavor = "multi_thread")]
async fn routes_handshake_requests() {
    let daemon = isolated_daemon();
    let response = daemon
        .handle_request(RequestEnvelope {
            operation_id: OperationId::new(),
            payload: RequestPayload::Handshake,
        })
        .await
        .expect("handshake routing should succeed");

    match response.payload {
        ResponsePayload::Handshake(handshake) => {
            assert_eq!(handshake.protocol_version.major, 0);
            assert_eq!(handshake.daemon_phase, terminal_protocol::DaemonPhase::Ready);
            assert_eq!(
                handshake.available_backends,
                crate::backend_registry::compiled_backend_kinds()
            );
            assert_eq!(handshake.binary_version, CURRENT_BINARY_VERSION.to_string());
            assert_eq!(handshake.protocol_version.major, CURRENT_PROTOCOL_MAJOR);
            assert_eq!(handshake.protocol_version.minor, CURRENT_PROTOCOL_MINOR);
            assert!(handshake.capabilities.request_reply);
            assert!(handshake.capabilities.topology_subscriptions);
            assert!(handshake.capabilities.pane_subscriptions);
            assert!(handshake.capabilities.backend_discovery);
            assert!(handshake.capabilities.backend_capability_queries);
            assert!(handshake.capabilities.saved_sessions);
            assert!(handshake.capabilities.session_restore);
            assert!(handshake.capabilities.degraded_error_reasons);
            assert!(handshake.capabilities.session_health);
        }
        other => panic!("unexpected payload: {other:?}"),
    }
}
