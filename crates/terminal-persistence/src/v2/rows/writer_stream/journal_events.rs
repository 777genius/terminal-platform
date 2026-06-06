use super::super::super::*;

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_journal_events)]
pub(in crate::v2) struct NewJournalEventRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) session_id: String,
    pub(in crate::v2) pane_id: Option<String>,
    pub(in crate::v2) commit_id: String,
    pub(in crate::v2) stream_id: String,
    pub(in crate::v2) event_scope_kind: String,
    pub(in crate::v2) event_scope_id: String,
    pub(in crate::v2) event_seq: i64,
    pub(in crate::v2) event_type: String,
    pub(in crate::v2) byte_low: Option<i64>,
    pub(in crate::v2) byte_high: Option<i64>,
    pub(in crate::v2) payload_json: Option<String>,
    pub(in crate::v2) payload_schema_id: Option<String>,
    pub(in crate::v2) source_event_id_hash: Option<String>,
    pub(in crate::v2) occurred_at_ms: i64,
    pub(in crate::v2) created_at_ms: i64,
    pub(in crate::v2) capture_semantics: String,
    pub(in crate::v2) trust_level: String,
    pub(in crate::v2) metadata_json: Option<String>,
}
