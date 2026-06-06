use napi::Result;
use serde_json::Value;
use terminal_node::NodeHostClient;

use crate::json::{protocol_error, to_json};

pub(super) async fn list_saved_sessions(client: NodeHostClient) -> Result<Value> {
    client.list_saved_sessions().await.map_err(protocol_error).and_then(to_json)
}

pub(super) async fn saved_session(client: NodeHostClient, session_id: String) -> Result<Value> {
    client.saved_session(&session_id).await.map_err(protocol_error).and_then(to_json)
}

pub(super) async fn delete_saved_session(
    client: NodeHostClient,
    session_id: String,
) -> Result<Value> {
    client.delete_saved_session(&session_id).await.map_err(protocol_error).and_then(to_json)
}

pub(super) async fn prune_saved_sessions(
    client: NodeHostClient,
    keep_latest: u32,
) -> Result<Value> {
    client
        .prune_saved_sessions(keep_latest as usize)
        .await
        .map_err(protocol_error)
        .and_then(to_json)
}

pub(super) async fn restore_saved_session(
    client: NodeHostClient,
    session_id: String,
) -> Result<Value> {
    client.restore_saved_session(&session_id).await.map_err(protocol_error).and_then(to_json)
}
