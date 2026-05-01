use super::super::super::*;

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_history_gaps)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
pub(in crate::v2) struct HistoryGapRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) session_id: String,
    pub(in crate::v2) pane_id: Option<String>,
    pub(in crate::v2) stream_id: String,
    pub(in crate::v2) gap_kind: String,
    pub(in crate::v2) event_seq_low: Option<i64>,
    pub(in crate::v2) event_seq_high: Option<i64>,
    pub(in crate::v2) byte_low: Option<i64>,
    pub(in crate::v2) byte_high: Option<i64>,
    pub(in crate::v2) estimated_dropped_bytes: Option<i64>,
    pub(in crate::v2) estimated_dropped_events: Option<i64>,
    pub(in crate::v2) reason: String,
    pub(in crate::v2) writer_generation: Option<String>,
    pub(in crate::v2) opened_at_ms: i64,
    pub(in crate::v2) closed_at_ms: Option<i64>,
    pub(in crate::v2) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_history_gaps)]
pub(in crate::v2) struct NewHistoryGapRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) session_id: String,
    pub(in crate::v2) pane_id: Option<String>,
    pub(in crate::v2) stream_id: String,
    pub(in crate::v2) gap_kind: String,
    pub(in crate::v2) event_seq_low: Option<i64>,
    pub(in crate::v2) event_seq_high: Option<i64>,
    pub(in crate::v2) byte_low: Option<i64>,
    pub(in crate::v2) byte_high: Option<i64>,
    pub(in crate::v2) estimated_dropped_bytes: Option<i64>,
    pub(in crate::v2) estimated_dropped_events: Option<i64>,
    pub(in crate::v2) reason: String,
    pub(in crate::v2) writer_generation: Option<String>,
    pub(in crate::v2) opened_at_ms: i64,
    pub(in crate::v2) closed_at_ms: Option<i64>,
    pub(in crate::v2) metadata_json: Option<String>,
}
