use super::super::*;

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_clients)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
pub(in crate::v2) struct DeliveryClientRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) client_kind: String,
    pub(in crate::v2) install_ref_hash: Option<String>,
    pub(in crate::v2) browser_profile_ref_hash: Option<String>,
    pub(in crate::v2) user_agent_hash: Option<String>,
    pub(in crate::v2) created_at_ms: i64,
    pub(in crate::v2) last_seen_at_ms: i64,
    pub(in crate::v2) trust_state: String,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_clients)]
pub(in crate::v2) struct NewDeliveryClientRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) client_kind: String,
    pub(in crate::v2) install_ref_hash: Option<String>,
    pub(in crate::v2) browser_profile_ref_hash: Option<String>,
    pub(in crate::v2) user_agent_hash: Option<String>,
    pub(in crate::v2) created_at_ms: i64,
    pub(in crate::v2) last_seen_at_ms: i64,
    pub(in crate::v2) trust_state: String,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_delivery_offsets)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
pub(in crate::v2) struct DeliveryOffsetRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) client_id: String,
    pub(in crate::v2) session_id: String,
    pub(in crate::v2) pane_id: Option<String>,
    pub(in crate::v2) stream_id: String,
    pub(in crate::v2) last_sent_event_seq: i64,
    pub(in crate::v2) last_acked_event_seq: i64,
    pub(in crate::v2) last_persisted_event_seq: i64,
    pub(in crate::v2) replay_from_event_seq: Option<i64>,
    pub(in crate::v2) gap_state: String,
    pub(in crate::v2) updated_at_ms: i64,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_delivery_offsets)]
pub(in crate::v2) struct NewDeliveryOffsetRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) client_id: String,
    pub(in crate::v2) session_id: String,
    pub(in crate::v2) pane_id: Option<String>,
    pub(in crate::v2) stream_id: String,
    pub(in crate::v2) last_sent_event_seq: i64,
    pub(in crate::v2) last_acked_event_seq: i64,
    pub(in crate::v2) last_persisted_event_seq: i64,
    pub(in crate::v2) replay_from_event_seq: Option<i64>,
    pub(in crate::v2) gap_state: String,
    pub(in crate::v2) updated_at_ms: i64,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_outbox_messages)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
pub(in crate::v2) struct OutboxMessageRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) message_kind: String,
    pub(in crate::v2) dedupe_key: Option<String>,
    pub(in crate::v2) state: String,
    pub(in crate::v2) payload_json: String,
    pub(in crate::v2) attempts: i64,
    pub(in crate::v2) max_attempts: i64,
    pub(in crate::v2) claimed_by: Option<String>,
    pub(in crate::v2) lease_token: Option<String>,
    pub(in crate::v2) claimed_until_ms: Option<i64>,
    pub(in crate::v2) next_run_at_ms: i64,
    pub(in crate::v2) last_error: Option<String>,
    pub(in crate::v2) created_at_ms: i64,
    pub(in crate::v2) updated_at_ms: i64,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_outbox_messages)]
pub(in crate::v2) struct NewOutboxMessageRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) message_kind: String,
    pub(in crate::v2) dedupe_key: Option<String>,
    pub(in crate::v2) state: String,
    pub(in crate::v2) payload_json: String,
    pub(in crate::v2) attempts: i64,
    pub(in crate::v2) max_attempts: i64,
    pub(in crate::v2) claimed_by: Option<String>,
    pub(in crate::v2) lease_token: Option<String>,
    pub(in crate::v2) claimed_until_ms: Option<i64>,
    pub(in crate::v2) next_run_at_ms: i64,
    pub(in crate::v2) last_error: Option<String>,
    pub(in crate::v2) created_at_ms: i64,
    pub(in crate::v2) updated_at_ms: i64,
}
