use super::super::super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryClientInput {
    pub id: Option<String>,
    pub client_kind: String,
    pub install_ref_hash: Option<String>,
    pub browser_profile_ref_hash: Option<String>,
    pub user_agent_hash: Option<String>,
    pub trust_state: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryOffsetInput {
    pub client_id: String,
    pub session_id: String,
    pub pane_id: String,
    pub stream_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryProgressInput {
    pub client_id: String,
    pub session_id: String,
    pub pane_id: String,
    pub stream_id: Option<String>,
    pub last_sent_event_seq: Option<i64>,
    pub last_acked_event_seq: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboxMessageInput {
    pub message_kind: String,
    pub payload: Value,
    pub dedupe_key: Option<String>,
    pub max_attempts: Option<i64>,
    pub next_run_at_ms: Option<i64>,
}
