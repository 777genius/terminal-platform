use std::path::PathBuf;

use napi::Result;
use napi_derive::napi;
use serde_json::Value;
use terminal_node::NodeHostClient;
use terminal_protocol::LocalSocketAddress;

use crate::subscription::TerminalNodeSubscriptionBinding;

mod connection;
mod history;
mod live_sessions;
mod saved_sessions;
mod subscriptions;

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
        connection::address(&self.inner)
    }

    #[napi(js_name = "bindingVersion")]
    pub fn binding_version(&self) -> Result<Value> {
        connection::binding_version(&self.inner)
    }

    #[napi(js_name = "handshakeInfo")]
    pub async fn handshake_info(&self) -> Result<Value> {
        connection::handshake_info(self.inner.clone()).await
    }

    #[napi(js_name = "listSessions")]
    pub async fn list_sessions(&self) -> Result<Value> {
        live_sessions::list_sessions(self.inner.clone()).await
    }

    #[napi(js_name = "discoverSessions")]
    pub async fn discover_sessions(&self, backend: Value) -> Result<Value> {
        live_sessions::discover_sessions(self.inner.clone(), backend).await
    }

    #[napi(js_name = "backendCapabilities")]
    pub async fn backend_capabilities(&self, backend: Value) -> Result<Value> {
        live_sessions::backend_capabilities(self.inner.clone(), backend).await
    }

    #[napi(js_name = "createNativeSession")]
    pub async fn create_native_session(&self, request: Value) -> Result<Value> {
        live_sessions::create_native_session(self.inner.clone(), request).await
    }

    #[napi(js_name = "importSession")]
    pub async fn import_session(&self, route: Value, title: Option<String>) -> Result<Value> {
        live_sessions::import_session(self.inner.clone(), route, title).await
    }

    #[napi(js_name = "attachSession")]
    pub async fn attach_session(&self, session_id: String) -> Result<Value> {
        live_sessions::attach_session(self.inner.clone(), session_id).await
    }

    #[napi(js_name = "sessionHealthSnapshot")]
    pub async fn session_health_snapshot(&self, session_id: String) -> Result<Value> {
        live_sessions::session_health_snapshot(self.inner.clone(), session_id).await
    }

    #[napi(js_name = "topologySnapshot")]
    pub async fn topology_snapshot(&self, session_id: String) -> Result<Value> {
        live_sessions::topology_snapshot(self.inner.clone(), session_id).await
    }

    #[napi(js_name = "screenSnapshot")]
    pub async fn screen_snapshot(&self, session_id: String, pane_id: String) -> Result<Value> {
        live_sessions::screen_snapshot(self.inner.clone(), session_id, pane_id).await
    }

    #[napi(js_name = "screenDelta")]
    pub async fn screen_delta(
        &self,
        session_id: String,
        pane_id: String,
        from_sequence: u32,
    ) -> Result<Value> {
        live_sessions::screen_delta(self.inner.clone(), session_id, pane_id, from_sequence).await
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
        history::pane_history(
            self.inner.clone(),
            session_id,
            pane_id,
            from_event_seq,
            max_segments,
            max_bytes,
        )
        .await
    }

    #[napi(js_name = "commandHistory")]
    pub async fn command_history(
        &self,
        session_id: Option<String>,
        limit: Option<i64>,
    ) -> Result<Value> {
        history::command_history(self.inner.clone(), session_id, limit).await
    }

    #[napi(js_name = "dispatchMuxCommand")]
    pub async fn dispatch_mux_command(&self, session_id: String, command: Value) -> Result<Value> {
        live_sessions::dispatch_mux_command(self.inner.clone(), session_id, command).await
    }

    #[napi(js_name = "listSavedSessions")]
    pub async fn list_saved_sessions(&self) -> Result<Value> {
        saved_sessions::list_saved_sessions(self.inner.clone()).await
    }

    #[napi(js_name = "savedSession")]
    pub async fn saved_session(&self, session_id: String) -> Result<Value> {
        saved_sessions::saved_session(self.inner.clone(), session_id).await
    }

    #[napi(js_name = "deleteSavedSession")]
    pub async fn delete_saved_session(&self, session_id: String) -> Result<Value> {
        saved_sessions::delete_saved_session(self.inner.clone(), session_id).await
    }

    #[napi(js_name = "pruneSavedSessions")]
    pub async fn prune_saved_sessions(&self, keep_latest: u32) -> Result<Value> {
        saved_sessions::prune_saved_sessions(self.inner.clone(), keep_latest).await
    }

    #[napi(js_name = "restoreSavedSession")]
    pub async fn restore_saved_session(&self, session_id: String) -> Result<Value> {
        saved_sessions::restore_saved_session(self.inner.clone(), session_id).await
    }

    #[napi(js_name = "openSubscription")]
    pub async fn open_subscription(
        &self,
        session_id: String,
        spec: Value,
    ) -> Result<TerminalNodeSubscriptionBinding> {
        subscriptions::open_subscription(self.inner.clone(), session_id, spec).await
    }
}
