use terminal_backend_api::{
    BackendRawOutputEvent, CreateSessionSpec, MuxBackendPort, MuxCommand, SendInputSpec,
};

use crate::NativeBackend;

use super::support::{cat_launch_spec, echo_input, wait_for_screen_line};

#[tokio::test]
async fn writes_input_into_live_pty_backed_session() {
    let backend = NativeBackend::default();
    let binding = backend
        .create_session(CreateSessionSpec {
            title: Some("shell".to_string()),
            launch: Some(cat_launch_spec()),
        })
        .await
        .expect("native session should be created");
    let session = backend
        .attach_session(binding.session_id, binding.route)
        .await
        .expect("attach_session should succeed");
    let topology = session.topology_snapshot().await.expect("topology should succeed");
    let pane_id = topology.tabs[0].focused_pane.expect("focused pane should exist");

    wait_for_screen_line(&*session, pane_id, "ready").await;
    let before = session.screen_snapshot(pane_id).await.expect("screen snapshot should succeed");
    let result = session
        .dispatch(MuxCommand::SendInput(SendInputSpec {
            pane_id,
            data: echo_input("hello from backend test"),
            client_event_id: None,
        }))
        .await
        .expect("send input should succeed");

    assert!(!result.changed);
    wait_for_screen_line(&*session, pane_id, "hello from backend test").await;
    let delta =
        session.screen_delta(pane_id, before.sequence).await.expect("screen delta should succeed");
    let patch = delta.patch.expect("delta patch should exist");

    assert_eq!(delta.pane_id, pane_id);
    assert_eq!(delta.from_sequence, before.sequence);
    assert!(delta.to_sequence > before.sequence);
    assert!(
        patch.line_updates.iter().any(|line| line.line.text.contains("hello from backend test"))
    );
    assert!(delta.full_replace.is_none());
}

#[tokio::test]
async fn streams_raw_output_for_live_pty_backed_session() {
    let backend = NativeBackend::default();
    let binding = backend
        .create_session(CreateSessionSpec {
            title: Some("shell".to_string()),
            launch: Some(cat_launch_spec()),
        })
        .await
        .expect("native session should be created");
    let session = backend
        .attach_session(binding.session_id, binding.route)
        .await
        .expect("attach_session should succeed");
    let topology = session.topology_snapshot().await.expect("topology should succeed");
    let pane_id = topology.tabs[0].focused_pane.expect("focused pane should exist");

    wait_for_screen_line(&*session, pane_id, "ready").await;
    let mut raw_subscription =
        session.subscribe_raw_output(pane_id).await.expect("raw output should subscribe");
    let marker = "hello from native raw output";
    session
        .dispatch(MuxCommand::SendInput(SendInputSpec {
            pane_id,
            data: echo_input(marker),
            client_event_id: None,
        }))
        .await
        .expect("send input should succeed");

    let mut payload = Vec::new();
    for _ in 0..40 {
        if let Some(BackendRawOutputEvent::Bytes(bytes)) = raw_subscription.events.recv().await {
            payload.extend(bytes.payload);
            if payload.windows(marker.len()).any(|window| window == marker.as_bytes()) {
                raw_subscription.cancel();
                return;
            }
        }
    }

    panic!("raw output never contained marker; payload={:?}", String::from_utf8_lossy(&payload));
}
