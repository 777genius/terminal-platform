use futures_util::{SinkExt as _, StreamExt as _};
use interprocess::local_socket::{tokio::Stream, traits::tokio::Stream as _};
use terminal_backend_api::CreateSessionSpec;
use terminal_domain::{BackendKind, OperationId};
use tokio_util::codec::{Framed, LengthDelimitedCodec};

use terminal_protocol::{
    CreateSessionRequest, CreateSessionResponse, Handshake, ListSessionsResponse,
    LocalSocketAddress, ProtocolError, ProtocolVersion, RequestEnvelope, RequestPayload,
    ResponsePayload, TransportResponse, decode_json_frame, encode_json_frame,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonClientInfo {
    pub expected_protocol: ProtocolVersion,
}

impl Default for DaemonClientInfo {
    fn default() -> Self {
        Self { expected_protocol: ProtocolVersion { major: 0, minor: 1 } }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalSocketDaemonClient {
    address: LocalSocketAddress,
    info: DaemonClientInfo,
}

impl LocalSocketDaemonClient {
    #[must_use]
    pub fn new(address: LocalSocketAddress) -> Self {
        Self { address, info: DaemonClientInfo::default() }
    }

    #[must_use]
    pub fn address(&self) -> &LocalSocketAddress {
        &self.address
    }

    #[must_use]
    pub fn info(&self) -> &DaemonClientInfo {
        &self.info
    }

    pub async fn handshake(&self) -> Result<Handshake, ProtocolError> {
        let response = self.send_request(RequestPayload::Handshake).await?;

        match response.payload {
            ResponsePayload::Handshake(handshake) => Ok(handshake),
            other => Err(ProtocolError::unexpected_payload("handshake", &other)),
        }
    }

    pub async fn list_sessions(&self) -> Result<ListSessionsResponse, ProtocolError> {
        let response = self.send_request(RequestPayload::ListSessions).await?;

        match response.payload {
            ResponsePayload::ListSessions(list) => Ok(list),
            other => Err(ProtocolError::unexpected_payload("list_sessions", &other)),
        }
    }

    pub async fn create_session(
        &self,
        backend: BackendKind,
        spec: CreateSessionSpec,
    ) -> Result<CreateSessionResponse, ProtocolError> {
        let response = self
            .send_request(RequestPayload::CreateSession(CreateSessionRequest { backend, spec }))
            .await?;

        match response.payload {
            ResponsePayload::CreateSession(created) => Ok(created),
            other => Err(ProtocolError::unexpected_payload("create_session", &other)),
        }
    }

    async fn send_request(
        &self,
        payload: RequestPayload,
    ) -> Result<terminal_protocol::ResponseEnvelope, ProtocolError> {
        let operation_id = OperationId::new();
        let request = RequestEnvelope { operation_id, payload };
        let encoded_request = encode_json_frame(&request)?;
        let stream = Stream::connect(
            self.address
                .to_name()
                .map_err(|error| ProtocolError::io("invalid_socket_name", &error))?,
        )
        .await
        .map_err(|error| ProtocolError::io("connect_failed", &error))?;
        let mut framed = Framed::new(stream, LengthDelimitedCodec::new());

        framed
            .send(encoded_request)
            .await
            .map_err(|error| ProtocolError::io("send_failed", &error))?;

        let frame = framed
            .next()
            .await
            .ok_or_else(|| ProtocolError::new("unexpected_eof", "daemon closed stream"))?
            .map_err(|error| ProtocolError::io("receive_failed", &error))?;
        let response = decode_json_frame::<TransportResponse>(&frame)?.into_result()?;

        if response.operation_id != operation_id {
            return Err(ProtocolError::new(
                "operation_mismatch",
                format!(
                    "expected response for operation {:?}, got {:?}",
                    operation_id, response.operation_id
                ),
            ));
        }

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use terminal_backend_api::CreateSessionSpec;
    use terminal_daemon::{TerminalDaemon, spawn_local_socket_server};
    use terminal_domain::BackendKind;

    use super::LocalSocketDaemonClient;

    fn unique_address(label: &str) -> terminal_protocol::LocalSocketAddress {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let slug = format!("terminal-platform-{label}-{}-{nanos}.sock", std::process::id());

        terminal_protocol::LocalSocketAddress::from_runtime_slug(slug)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn roundtrips_handshake_and_empty_list_sessions() {
        let address = unique_address("daemon-client");
        let server = spawn_local_socket_server(TerminalDaemon::default(), address.clone())
            .expect("server should bind");
        let client = LocalSocketDaemonClient::new(address);

        let handshake = client.handshake().await.expect("handshake should succeed");
        let sessions = client.list_sessions().await.expect("list_sessions should succeed");

        assert_eq!(handshake.protocol_version.major, 0);
        assert_eq!(handshake.protocol_version.minor, 1);
        assert_eq!(client.info().expected_protocol, handshake.protocol_version);
        assert!(sessions.sessions.is_empty());

        server.shutdown().await.expect("server shutdown should succeed");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn creates_native_session_and_lists_it_back() {
        let address = unique_address("daemon-client-create");
        let server = spawn_local_socket_server(TerminalDaemon::default(), address.clone())
            .expect("server should bind");
        let client = LocalSocketDaemonClient::new(address);

        let created = client
            .create_session(
                BackendKind::Native,
                CreateSessionSpec { title: Some("shell".to_string()) },
            )
            .await
            .expect("create_session should succeed");
        let sessions = client.list_sessions().await.expect("list_sessions should succeed");

        assert_eq!(created.session.route.backend, BackendKind::Native);
        assert_eq!(created.session.title.as_deref(), Some("shell"));
        assert_eq!(sessions.sessions.len(), 1);
        assert_eq!(sessions.sessions[0].session_id, created.session.session_id);

        server.shutdown().await.expect("server shutdown should succeed");
    }
}
