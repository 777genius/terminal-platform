use napi::Result;
use napi_derive::napi;
use serde_json::Value;
use terminal_node::NodeSubscriptionHandle;

use crate::json::{protocol_error, to_json};

#[napi(js_name = "TerminalNodeSubscription")]
pub struct TerminalNodeSubscriptionBinding {
    pub(crate) inner: NodeSubscriptionHandle,
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
