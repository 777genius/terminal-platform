use serde::{Deserialize, Serialize};

use terminal_domain::{OperationId, SubscriptionId};

use crate::{RequestPayload, ResponsePayload, SubscriptionEvent};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestEnvelope {
    pub operation_id: OperationId,
    pub payload: RequestPayload,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseEnvelope {
    pub operation_id: OperationId,
    pub payload: ResponsePayload,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscriptionEnvelope {
    pub subscription_id: SubscriptionId,
    pub event: SubscriptionEvent,
}
