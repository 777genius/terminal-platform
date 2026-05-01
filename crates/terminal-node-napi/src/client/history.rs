use napi::Result;
use serde_json::Value;
use terminal_node::NodeHostClient;

use crate::json::{protocol_error, to_json};

pub(super) async fn pane_history(
    client: NodeHostClient,
    session_id: String,
    pane_id: String,
    from_event_seq: Option<i64>,
    max_segments: Option<i64>,
    max_bytes: Option<i64>,
) -> Result<Value> {
    client
        .pane_history(&session_id, &pane_id, from_event_seq, max_segments, max_bytes)
        .await
        .map_err(protocol_error)
        .and_then(to_json)
}

pub(super) async fn command_history(
    client: NodeHostClient,
    session_id: Option<String>,
    limit: Option<i64>,
) -> Result<Value> {
    client
        .command_history(session_id.as_deref(), limit)
        .await
        .map_err(protocol_error)
        .and_then(to_json)
}
