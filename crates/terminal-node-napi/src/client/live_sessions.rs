use napi::Result;
use serde_json::Value;
use terminal_node::NodeHostClient;

use crate::json::{from_json, protocol_error, to_json};

pub(super) async fn list_sessions(client: NodeHostClient) -> Result<Value> {
    client.list_sessions().await.map_err(protocol_error).and_then(to_json)
}

pub(super) async fn discover_sessions(client: NodeHostClient, backend: Value) -> Result<Value> {
    let backend = from_json(backend, "invalid_backend_kind")?;
    client.discover_sessions(backend).await.map_err(protocol_error).and_then(to_json)
}

pub(super) async fn backend_capabilities(client: NodeHostClient, backend: Value) -> Result<Value> {
    let backend = from_json(backend, "invalid_backend_kind")?;
    client.backend_capabilities(backend).await.map_err(protocol_error).and_then(to_json)
}

pub(super) async fn create_native_session(client: NodeHostClient, request: Value) -> Result<Value> {
    let request = from_json(request, "invalid_create_session_request")?;
    client.create_native_session(&request).await.map_err(protocol_error).and_then(to_json)
}

pub(super) async fn import_session(
    client: NodeHostClient,
    route: Value,
    title: Option<String>,
) -> Result<Value> {
    let route = from_json(route, "invalid_session_route")?;
    client.import_session(&route, title).await.map_err(protocol_error).and_then(to_json)
}

pub(super) async fn attach_session(client: NodeHostClient, session_id: String) -> Result<Value> {
    client.attach_session(&session_id).await.map_err(protocol_error).and_then(to_json)
}

pub(super) async fn session_health_snapshot(
    client: NodeHostClient,
    session_id: String,
) -> Result<Value> {
    client.session_health_snapshot(&session_id).await.map_err(protocol_error).and_then(to_json)
}

pub(super) async fn topology_snapshot(client: NodeHostClient, session_id: String) -> Result<Value> {
    client.topology_snapshot(&session_id).await.map_err(protocol_error).and_then(to_json)
}

pub(super) async fn screen_snapshot(
    client: NodeHostClient,
    session_id: String,
    pane_id: String,
) -> Result<Value> {
    client.screen_snapshot(&session_id, &pane_id).await.map_err(protocol_error).and_then(to_json)
}

pub(super) async fn screen_delta(
    client: NodeHostClient,
    session_id: String,
    pane_id: String,
    from_sequence: u32,
) -> Result<Value> {
    client
        .screen_delta(&session_id, &pane_id, u64::from(from_sequence))
        .await
        .map_err(protocol_error)
        .and_then(to_json)
}

pub(super) async fn dispatch_mux_command(
    client: NodeHostClient,
    session_id: String,
    command: Value,
) -> Result<Value> {
    let command = from_json(command, "invalid_mux_command")?;
    client
        .dispatch_mux_command(&session_id, &command)
        .await
        .map_err(protocol_error)
        .and_then(to_json)
}
