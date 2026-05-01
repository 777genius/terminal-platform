use napi::Result;
use serde_json::Value;
use terminal_node::NodeHostClient;

use crate::json::{protocol_error, to_json};

pub(super) fn address(client: &NodeHostClient) -> String {
    client.address().to_string()
}

pub(super) fn binding_version(client: &NodeHostClient) -> Result<Value> {
    to_json(client.binding_version())
}

pub(super) async fn handshake_info(client: NodeHostClient) -> Result<Value> {
    client.handshake_info().await.map_err(protocol_error).and_then(to_json)
}
