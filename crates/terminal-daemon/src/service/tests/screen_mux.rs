use super::{prelude::*, support::*};

#[cfg(feature = "native-backend")]
#[tokio::test(flavor = "multi_thread")]
async fn routes_topology_screen_and_subscription_requests() {
    let daemon = isolated_daemon();
    let created = daemon
        .handle_request(RequestEnvelope {
            operation_id: OperationId::new(),
            payload: RequestPayload::CreateSession(terminal_protocol::CreateSessionRequest {
                backend: terminal_domain::BackendKind::Native,
                spec: CreateSessionSpec {
                    title: Some("screen-shell".to_string()),
                    ..CreateSessionSpec::default()
                },
            }),
        })
        .await
        .expect("create session should succeed");
    let session_id = match created.payload {
        ResponsePayload::CreateSession(created) => created.session.session_id,
        other => panic!("unexpected payload: {other:?}"),
    };

    let topology = daemon
        .handle_request(RequestEnvelope {
            operation_id: OperationId::new(),
            payload: RequestPayload::GetTopologySnapshot(
                terminal_protocol::GetTopologySnapshotRequest { session_id },
            ),
        })
        .await
        .expect("topology snapshot should succeed");
    let pane_id = match topology.payload {
        ResponsePayload::TopologySnapshot(snapshot) => {
            snapshot.tabs[0].focused_pane.expect("focused pane should exist")
        }
        other => panic!("unexpected payload: {other:?}"),
    };

    let screen = daemon
        .handle_request(RequestEnvelope {
            operation_id: OperationId::new(),
            payload: RequestPayload::GetScreenSnapshot(
                terminal_protocol::GetScreenSnapshotRequest { session_id, pane_id },
            ),
        })
        .await
        .expect("screen snapshot should succeed");
    let sequence = match screen.payload {
        ResponsePayload::ScreenSnapshot(snapshot) => snapshot.sequence,
        other => panic!("unexpected payload: {other:?}"),
    };

    let delta = daemon
        .handle_request(RequestEnvelope {
            operation_id: OperationId::new(),
            payload: RequestPayload::GetScreenDelta(terminal_protocol::GetScreenDeltaRequest {
                session_id,
                pane_id,
                from_sequence: sequence,
            }),
        })
        .await
        .expect("screen delta should succeed");
    assert!(matches!(delta.payload, ResponsePayload::ScreenDelta(_)));

    let subscription = daemon
        .open_subscription(terminal_protocol::OpenSubscriptionRequest {
            session_id,
            spec: SubscriptionSpec::SessionTopology,
        })
        .await
        .expect("open subscription should succeed");
    assert!(!subscription.subscription_id.0.as_hyphenated().to_string().is_empty());
}

#[cfg(feature = "native-backend")]
#[tokio::test(flavor = "multi_thread")]
async fn routes_dispatch_mux_command_requests() {
    let daemon = isolated_daemon();
    let created = daemon
        .handle_request(RequestEnvelope {
            operation_id: OperationId::new(),
            payload: RequestPayload::CreateSession(terminal_protocol::CreateSessionRequest {
                backend: terminal_domain::BackendKind::Native,
                spec: CreateSessionSpec {
                    title: Some("mux-shell".to_string()),
                    ..CreateSessionSpec::default()
                },
            }),
        })
        .await
        .expect("create session should succeed");
    let session_id = match created.payload {
        ResponsePayload::CreateSession(created) => created.session.session_id,
        other => panic!("unexpected payload: {other:?}"),
    };

    let response = daemon
        .handle_request(RequestEnvelope {
            operation_id: OperationId::new(),
            payload: RequestPayload::DispatchMuxCommand(
                terminal_protocol::DispatchMuxCommandRequest {
                    session_id,
                    command: MuxCommand::NewTab(NewTabSpec { title: Some("logs".to_string()) }),
                },
            ),
        })
        .await
        .expect("dispatch mux command should succeed");

    match response.payload {
        ResponsePayload::DispatchMuxCommand(result) => {
            assert!(result.changed);
        }
        other => panic!("unexpected payload: {other:?}"),
    }
}
