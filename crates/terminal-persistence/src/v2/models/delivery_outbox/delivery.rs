use super::super::super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliveryClientRecord {
    pub id: String,
    pub client_kind: String,
    pub last_seen_at_ms: i64,
    pub trust_state: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliveryOffsetRecord {
    pub id: String,
    pub client_id: String,
    pub session_id: String,
    pub pane_id: Option<String>,
    pub stream_id: String,
    pub last_sent_event_seq: i64,
    pub last_acked_event_seq: i64,
    pub last_persisted_event_seq: i64,
    pub replay_from_event_seq: Option<i64>,
    pub gap_state: String,
    pub updated_at_ms: i64,
}

impl From<DeliveryOffsetRow> for DeliveryOffsetRecord {
    fn from(row: DeliveryOffsetRow) -> Self {
        Self {
            id: row.id,
            client_id: row.client_id,
            session_id: row.session_id,
            pane_id: row.pane_id,
            stream_id: row.stream_id,
            last_sent_event_seq: row.last_sent_event_seq,
            last_acked_event_seq: row.last_acked_event_seq,
            last_persisted_event_seq: row.last_persisted_event_seq,
            replay_from_event_seq: row.replay_from_event_seq,
            gap_state: row.gap_state,
            updated_at_ms: row.updated_at_ms,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliveryReplayWindow {
    pub from_event_seq: Option<i64>,
    pub to_event_seq: i64,
    pub gap_state: String,
}
