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
    let handshake = fixture.client.handshake().await.expect("handshake should succeed");
    let sessions = fixture.client.list_sessions().await.expect("list_sessions should succeed");

    assert_eq!(handshake.protocol_version.major, 0);
    assert_eq!(handshake.protocol_version.minor, 1);
    assert_eq!(sessions.sessions.len(), 0);

    fixture.shutdown().await.expect("fixture should stop cleanly");
}
