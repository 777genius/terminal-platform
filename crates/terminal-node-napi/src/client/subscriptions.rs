use napi::Result;
use serde_json::Value;
use terminal_node::NodeHostClient;

use crate::{
    json::{from_json, protocol_error},
    subscription::TerminalNodeSubscriptionBinding,
};

pub(super) async fn open_subscription(
    client: NodeHostClient,
    session_id: String,
    spec: Value,
) -> Result<TerminalNodeSubscriptionBinding> {
    let spec = from_json(spec, "invalid_subscription_spec")?;
    let subscription =
        client.open_subscription(&session_id, &spec).await.map_err(protocol_error)?;

    Ok(TerminalNodeSubscriptionBinding { inner: subscription })
}
