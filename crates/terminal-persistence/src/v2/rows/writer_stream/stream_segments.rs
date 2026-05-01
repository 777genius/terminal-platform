use super::super::super::*;

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_stream_segments)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
pub(in crate::v2) struct StreamSegmentRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) session_id: String,
    pub(in crate::v2) pane_id: String,
    pub(in crate::v2) commit_id: String,
    pub(in crate::v2) stream_id: String,
    pub(in crate::v2) event_seq_low: i64,
    pub(in crate::v2) event_seq_high: i64,
    pub(in crate::v2) byte_low: i64,
    pub(in crate::v2) byte_high: i64,
    pub(in crate::v2) payload: Vec<u8>,
    pub(in crate::v2) payload_len: i64,
    pub(in crate::v2) stored_byte_len: i64,
    pub(in crate::v2) uncompressed_byte_len: Option<i64>,
    pub(in crate::v2) checksum_algorithm: String,
    pub(in crate::v2) checksum: String,
    pub(in crate::v2) compression: String,
    pub(in crate::v2) capture_semantics: String,
    pub(in crate::v2) encryption_state: String,
    pub(in crate::v2) key_ref: Option<String>,
    pub(in crate::v2) created_at_ms: i64,
    pub(in crate::v2) writer_generation: String,
    pub(in crate::v2) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_stream_segments)]
pub(in crate::v2) struct NewStreamSegmentRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) session_id: String,
    pub(in crate::v2) pane_id: String,
    pub(in crate::v2) commit_id: String,
    pub(in crate::v2) stream_id: String,
    pub(in crate::v2) event_seq_low: i64,
    pub(in crate::v2) event_seq_high: i64,
    pub(in crate::v2) byte_low: i64,
    pub(in crate::v2) byte_high: i64,
    pub(in crate::v2) payload: Vec<u8>,
    pub(in crate::v2) payload_len: i64,
    pub(in crate::v2) stored_byte_len: i64,
    pub(in crate::v2) uncompressed_byte_len: Option<i64>,
    pub(in crate::v2) checksum_algorithm: String,
    pub(in crate::v2) checksum: String,
    pub(in crate::v2) compression: String,
    pub(in crate::v2) capture_semantics: String,
    pub(in crate::v2) encryption_state: String,
    pub(in crate::v2) key_ref: Option<String>,
    pub(in crate::v2) created_at_ms: i64,
    pub(in crate::v2) writer_generation: String,
    pub(in crate::v2) metadata_json: Option<String>,
}
