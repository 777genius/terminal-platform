use super::{prelude::*, support::*};

#[cfg(feature = "native-backend")]
#[tokio::test(flavor = "multi_thread")]
async fn routes_create_and_list_session_requests() {
    let daemon = isolated_daemon();
    let create = daemon
        .handle_request(RequestEnvelope {
            operation_id: OperationId::new(),
            payload: RequestPayload::CreateSession(terminal_protocol::CreateSessionRequest {
                backend: terminal_domain::BackendKind::Native,
                spec: CreateSessionSpec {
                    title: Some("shell".to_string()),
                    ..CreateSessionSpec::default()
                },
            }),
        })
        .await
        .expect("create session should succeed");

    match create.payload {
        ResponsePayload::CreateSession(created) => {
            assert_eq!(created.session.title.as_deref(), Some("shell"));
            assert_eq!(created.session.route.backend, terminal_domain::BackendKind::Native);
        }
        other => panic!("unexpected payload: {other:?}"),
    }

    let listed = daemon
        .handle_request(RequestEnvelope {
            operation_id: OperationId::new(),
            payload: RequestPayload::ListSessions,
        })
        .await
        .expect("list sessions should succeed");

    match listed.payload {
        ResponsePayload::ListSessions(list) => {
            assert_eq!(list.sessions.len(), 1);
            assert_eq!(list.sessions[0].title.as_deref(), Some("shell"));
        }
        other => panic!("unexpected payload: {other:?}"),
    }
}

#[cfg(feature = "native-backend")]
#[tokio::test(flavor = "multi_thread")]
async fn routes_backend_capabilities_requests() {
    let backend = terminal_domain::BackendKind::Native;
    let daemon = TerminalDaemon::default();
    let response = daemon
        .handle_request(RequestEnvelope {
            operation_id: OperationId::new(),
            payload: RequestPayload::GetBackendCapabilities(
                terminal_protocol::GetBackendCapabilitiesRequest { backend },
            ),
        })
        .await
        .expect("capabilities routing should succeed");

    match response.payload {
        ResponsePayload::BackendCapabilities(capabilities) => {
            assert_eq!(capabilities.backend, backend);
        }
        other => panic!("unexpected payload: {other:?}"),
    }
}
