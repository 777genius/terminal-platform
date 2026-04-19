use terminal_backend_api::{CreateSessionSpec, MuxCommand, NewTabSpec};
use terminal_domain::BackendKind;
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
        .create_session(BackendKind::Native, CreateSessionSpec { title: Some("shell".to_string()) })
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
    assert_eq!(screen.surface.lines.len(), 2);
    assert!(dispatch.changed);
    assert_eq!(topology_after_dispatch.tabs.len(), 2);

    fixture.shutdown().await.expect("fixture should stop cleanly");
}
