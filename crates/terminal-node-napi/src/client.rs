use std::path::PathBuf;

use napi::Result;
use napi_derive::napi;
use serde_json::Value;
use terminal_node::NodeHostClient;
use terminal_protocol::LocalSocketAddress;

use crate::{
    json::{from_json, protocol_error, to_json},
    subscription::TerminalNodeSubscriptionBinding,
};

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

    #[napi(js_name = "sessionHealthSnapshot")]
    pub async fn session_health_snapshot(&self, session_id: String) -> Result<Value> {
        let client = self.inner.clone();
        client.session_health_snapshot(&session_id).await.map_err(protocol_error).and_then(to_json)
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

    #[napi(js_name = "paneHistory")]
    pub async fn pane_history(
        &self,
        session_id: String,
        pane_id: String,
        from_event_seq: Option<i64>,
        max_segments: Option<i64>,
        max_bytes: Option<i64>,
    ) -> Result<Value> {
        let client = self.inner.clone();
        client
            .pane_history(&session_id, &pane_id, from_event_seq, max_segments, max_bytes)
            .await
            .map_err(protocol_error)
            .and_then(to_json)
    }

    #[napi(js_name = "commandHistory")]
    pub async fn command_history(
        &self,
        session_id: Option<String>,
        limit: Option<i64>,
    ) -> Result<Value> {
        let client = self.inner.clone();
        client
            .command_history(session_id.as_deref(), limit)
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
