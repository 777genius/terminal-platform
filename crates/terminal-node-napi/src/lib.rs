//! Thin Node/Electron leaf adapter over the safe `terminal-node` facade.

use std::path::PathBuf;

use napi::{Error, Result, Status};
use napi_derive::napi;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use terminal_node::{NodeHostClient, NodeSubscriptionHandle};
use terminal_protocol::{LocalSocketAddress, ProtocolError};

#[napi(js_name = "TerminalNodeClient")]
pub struct TerminalNodeBinding {
    inner: NodeHostClient,
}

#[napi(js_name = "TerminalNodeSubscription")]
pub struct TerminalNodeSubscriptionBinding {
    inner: NodeSubscriptionHandle,
}

#[napi]
impl TerminalNodeSubscriptionBinding {
    #[napi(getter, js_name = "subscriptionId")]
    pub fn subscription_id(&self) -> String {
        self.inner.meta().subscription_id
    }

    #[napi(js_name = "nextEvent")]
    pub async fn next_event(&self) -> Result<Value> {
        let event = self.inner.next_event().await.map_err(protocol_error)?;
        match event {
            Some(event) => to_json(event),
            None => Ok(Value::Null),
        }
    }

    #[napi]
    pub async fn close(&self) {
        self.inner.close().await;
    }
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

    #[napi(js_name = "listSavedSessions")]
    pub async fn list_saved_sessions(&self) -> Result<Value> {
        let client = self.inner.clone();
        client.list_saved_sessions().await.map_err(protocol_error).and_then(to_json)
    }

    #[napi(js_name = "discoverSessions")]
    pub async fn discover_sessions(&self, backend: Value) -> Result<Value> {
        let client = self.inner.clone();
        let backend = from_json(backend, "invalid_backend_kind")?;
        client.discover_sessions(backend).await.map_err(protocol_error).and_then(to_json)
    }

    #[napi(js_name = "backendCapabilities")]
    pub async fn backend_capabilities(&self, backend: Value) -> Result<Value> {
        let client = self.inner.clone();
        let backend = from_json(backend, "invalid_backend_kind")?;
        client.backend_capabilities(backend).await.map_err(protocol_error).and_then(to_json)
    }

    #[napi(js_name = "createNativeSession")]
    pub async fn create_native_session(&self, request: Value) -> Result<Value> {
        let client = self.inner.clone();
        let request = from_json(request, "invalid_create_session_request")?;
        client.create_native_session(&request).await.map_err(protocol_error).and_then(to_json)
    }

    #[napi(js_name = "importSession")]
    pub async fn import_session(&self, route: Value, title: Option<String>) -> Result<Value> {
        let client = self.inner.clone();
        let route = from_json(route, "invalid_session_route")?;
        client.import_session(&route, title).await.map_err(protocol_error).and_then(to_json)
    }

    #[napi(js_name = "savedSession")]
    pub async fn saved_session(&self, session_id: String) -> Result<Value> {
        let client = self.inner.clone();
        client.saved_session(&session_id).await.map_err(protocol_error).and_then(to_json)
    }

    #[napi(js_name = "deleteSavedSession")]
    pub async fn delete_saved_session(&self, session_id: String) -> Result<Value> {
        let client = self.inner.clone();
        client.delete_saved_session(&session_id).await.map_err(protocol_error).and_then(to_json)
    }

    #[napi(js_name = "pruneSavedSessions")]
    pub async fn prune_saved_sessions(&self, keep_latest: u32) -> Result<Value> {
        let client = self.inner.clone();
        client
            .prune_saved_sessions(keep_latest as usize)
            .await
            .map_err(protocol_error)
            .and_then(to_json)
    }

    #[napi(js_name = "restoreSavedSession")]
    pub async fn restore_saved_session(&self, session_id: String) -> Result<Value> {
        let client = self.inner.clone();
        client.restore_saved_session(&session_id).await.map_err(protocol_error).and_then(to_json)
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

    #[napi(js_name = "screenDelta")]
    pub async fn screen_delta(
        &self,
        session_id: String,
        pane_id: String,
        from_sequence: u32,
    ) -> Result<Value> {
        let client = self.inner.clone();
        client
            .screen_delta(&session_id, &pane_id, u64::from(from_sequence))
            .await
            .map_err(protocol_error)
            .and_then(to_json)
    }

    #[napi(js_name = "dispatchMuxCommand")]
    pub async fn dispatch_mux_command(&self, session_id: String, command: Value) -> Result<Value> {
        let client = self.inner.clone();
        let command = from_json(command, "invalid_mux_command")?;
        client
            .dispatch_mux_command(&session_id, &command)
            .await
            .map_err(protocol_error)
            .and_then(to_json)
    }

    #[napi(js_name = "openSubscription")]
    pub async fn open_subscription(
        &self,
        session_id: String,
        spec: Value,
    ) -> Result<TerminalNodeSubscriptionBinding> {
        let client = self.inner.clone();
        let spec = from_json(spec, "invalid_subscription_spec")?;
        let subscription =
            client.open_subscription(&session_id, &spec).await.map_err(protocol_error)?;

        Ok(TerminalNodeSubscriptionBinding { inner: subscription })
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
    fn deserializes_session_requests_with_null_launch() {
        let request = from_json::<terminal_node::NodeCreateSessionRequest>(
            serde_json::json!({
                "title": "shell",
                "launch": null
            }),
            "invalid_create_session_request",
        )
        .expect("json decoding should succeed");

        assert_eq!(request.title.as_deref(), Some("shell"));
        assert!(request.launch.is_none());
    }

    #[test]
    fn prefixes_protocol_codes_into_napi_errors() {
        let error = code_error("invalid_session_id", "bad session id");

        assert_eq!(error.reason, "invalid_session_id: bad session id");
    }
}
