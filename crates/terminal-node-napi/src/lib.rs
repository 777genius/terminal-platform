//! Thin Node/Electron leaf adapter over the safe `terminal-node` facade.

use std::path::PathBuf;

use napi::{Error, Result, Status};
use napi_derive::napi;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use terminal_node::NodeHostClient;
use terminal_protocol::{LocalSocketAddress, ProtocolError};

#[napi(js_name = "TerminalNodeClient")]
pub struct TerminalNodeBinding {
    inner: NodeHostClient,
}

#[napi]
impl TerminalNodeBinding {
    #[napi(factory, js_name = "fromRuntimeSlug")]
    pub fn from_runtime_slug(slug: String) -> Self {
        Self { inner: NodeHostClient::from_runtime_slug(slug) }
    }

    #[napi(factory, js_name = "fromNamespacedAddress")]
    pub fn from_namespaced_address(value: String) -> Self {
        Self { inner: NodeHostClient::new(LocalSocketAddress::Namespaced(value)) }
    }

    #[napi(factory, js_name = "fromFilesystemPath")]
    pub fn from_filesystem_path(path: String) -> Self {
        Self { inner: NodeHostClient::new(LocalSocketAddress::Filesystem(PathBuf::from(path))) }
    }

    #[napi(getter)]
    pub fn address(&self) -> String {
        self.inner.address().to_string()
    }

    #[napi(js_name = "bindingVersion")]
    pub fn binding_version(&self) -> Result<Value> {
        to_json(self.inner.binding_version())
    }

    #[napi(js_name = "handshakeInfo")]
    pub async fn handshake_info(&self) -> Result<Value> {
        let client = self.inner.clone();
        client.handshake_info().await.map_err(protocol_error).and_then(to_json)
    }

    #[napi(js_name = "listSessions")]
    pub async fn list_sessions(&self) -> Result<Value> {
        let client = self.inner.clone();
        client.list_sessions().await.map_err(protocol_error).and_then(to_json)
    }

    #[napi(js_name = "createNativeSession")]
    pub async fn create_native_session(&self, request: Value) -> Result<Value> {
        let client = self.inner.clone();
        let request = from_json(request, "invalid_create_session_request")?;
        client.create_native_session(&request).await.map_err(protocol_error).and_then(to_json)
    }

    #[napi(js_name = "attachSession")]
    pub async fn attach_session(&self, session_id: String) -> Result<Value> {
        let client = self.inner.clone();
        client.attach_session(&session_id).await.map_err(protocol_error).and_then(to_json)
    }

    #[napi(js_name = "topologySnapshot")]
    pub async fn topology_snapshot(&self, session_id: String) -> Result<Value> {
        let client = self.inner.clone();
        client.topology_snapshot(&session_id).await.map_err(protocol_error).and_then(to_json)
    }

    #[napi(js_name = "screenSnapshot")]
    pub async fn screen_snapshot(&self, session_id: String, pane_id: String) -> Result<Value> {
        let client = self.inner.clone();
        client
            .screen_snapshot(&session_id, &pane_id)
            .await
            .map_err(protocol_error)
            .and_then(to_json)
    }
}

fn to_json<T>(value: T) -> Result<Value>
where
    T: Serialize,
{
    serde_json::to_value(value).map_err(|error| code_error("serialize_failed", error.to_string()))
}

fn from_json<T>(value: Value, code: &'static str) -> Result<T>
where
    T: DeserializeOwned,
{
    serde_json::from_value(value).map_err(|error| code_error(code, error.to_string()))
}

fn protocol_error(error: ProtocolError) -> Error {
    code_error(&error.code, error.message)
}

fn code_error(code: &str, message: impl Into<String>) -> Error {
    Error::new(Status::GenericFailure, format!("{code}: {}", message.into()))
}

#[cfg(test)]
mod tests {
    use super::{code_error, from_json, to_json};

    #[test]
    fn serializes_structured_payloads_to_json_values() {
        let value = to_json(vec!["alpha", "beta"]).expect("json conversion should succeed");

        assert_eq!(value, serde_json::json!(["alpha", "beta"]));
    }

    #[test]
    fn deserializes_session_requests_from_json_values() {
        let request = from_json::<terminal_node::NodeCreateSessionRequest>(
            serde_json::json!({
                "title": "shell",
                "launch": {
                    "program": "/bin/zsh",
                    "args": ["-i"],
                    "cwd": "/tmp"
                }
            }),
            "invalid_create_session_request",
        )
        .expect("json decoding should succeed");

        assert_eq!(request.title.as_deref(), Some("shell"));
        assert_eq!(request.launch.expect("launch should exist").program, "/bin/zsh".to_string());
    }

    #[test]
    fn prefixes_protocol_codes_into_napi_errors() {
        let error = code_error("invalid_session_id", "bad session id");

        assert_eq!(error.reason, "invalid_session_id: bad session id");
    }
}
