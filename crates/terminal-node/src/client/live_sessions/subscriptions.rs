use terminal_protocol::ProtocolError;

use crate::{NodeHostClient, dto::*, ids::parse_session_id, subscription::NodeSubscriptionHandle};

impl NodeHostClient {
    pub async fn open_subscription(
        &self,
        session_id: &str,
        spec: &NodeSubscriptionSpec,
    ) -> Result<NodeSubscriptionHandle, ProtocolError> {
        NodeSubscriptionHandle::open(self.client.clone(), parse_session_id(session_id)?, spec).await
    }
}
