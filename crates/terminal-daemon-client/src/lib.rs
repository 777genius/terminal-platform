use terminal_daemon::TerminalDaemon;
use terminal_domain::OperationId;
use terminal_protocol::{
    Handshake, ListSessionsResponse, ProtocolError, ProtocolVersion, RequestEnvelope,
    RequestPayload, ResponsePayload,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonClientInfo {
    pub expected_protocol: ProtocolVersion,
}

#[derive(Debug, Default)]
pub struct InProcessDaemonClient {
    daemon: TerminalDaemon,
}

impl InProcessDaemonClient {
    #[must_use]
    pub fn new(daemon: TerminalDaemon) -> Self {
        Self { daemon }
    }

    pub fn handshake(&self) -> Result<Handshake, ProtocolError> {
        let response = self.daemon.handle_request(RequestEnvelope {
            operation_id: OperationId::new(),
            payload: RequestPayload::Handshake,
        })?;

        match response.payload {
            ResponsePayload::Handshake(handshake) => Ok(handshake),
            other => Err(ProtocolError {
                code: "unexpected_payload".to_string(),
                message: format!("expected handshake, got {other:?}"),
            }),
        }
    }

    pub fn list_sessions(&self) -> Result<ListSessionsResponse, ProtocolError> {
        let response = self.daemon.handle_request(RequestEnvelope {
            operation_id: OperationId::new(),
            payload: RequestPayload::ListSessions,
        })?;

        match response.payload {
            ResponsePayload::ListSessions(list) => Ok(list),
            other => Err(ProtocolError {
                code: "unexpected_payload".to_string(),
                message: format!("expected list_sessions, got {other:?}"),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use terminal_daemon::TerminalDaemon;

    use super::InProcessDaemonClient;

    #[test]
    fn roundtrips_handshake_and_empty_list_sessions() {
        let client = InProcessDaemonClient::new(TerminalDaemon::default());

        let handshake = client.handshake().expect("handshake should succeed");
        let sessions = client.list_sessions().expect("list_sessions should succeed");

        assert_eq!(handshake.protocol_version.major, 0);
        assert_eq!(handshake.available_backends.len(), 3);
        assert!(sessions.sessions.is_empty());
    }
}
