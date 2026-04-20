use terminal_domain::BackendKind;
use terminal_protocol::{DaemonCapabilities, DaemonPhase, Handshake, ProtocolVersion};

#[test]
fn handshake_roundtrips_through_json() {
    let handshake = Handshake {
        protocol_version: ProtocolVersion { major: 0, minor: 1 },
        binary_version: "0.1.0-dev".to_string(),
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
        },
        available_backends: vec![BackendKind::Native, BackendKind::Tmux],
        session_scope: "current_user".to_string(),
    };

    let json = serde_json::to_string(&handshake).expect("handshake should serialize");
    let restored: Handshake =
        serde_json::from_str(&json).expect("handshake should deserialize back");

    assert_eq!(restored, handshake);
}
