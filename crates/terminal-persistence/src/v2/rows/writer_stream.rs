use super::super::*;

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_writer_generations)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
pub(in crate::v2) struct WriterGenerationRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) process_id: String,
    pub(in crate::v2) lease_token: String,
    pub(in crate::v2) state: String,
    pub(in crate::v2) acquired_at_ms: i64,
    pub(in crate::v2) heartbeat_at_ms: i64,
    pub(in crate::v2) lease_expires_at_ms: i64,
    pub(in crate::v2) released_at_ms: Option<i64>,
    pub(in crate::v2) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_writer_generations)]
pub(in crate::v2) struct NewWriterGenerationRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) process_id: String,
    pub(in crate::v2) lease_token: String,
    pub(in crate::v2) state: String,
    pub(in crate::v2) acquired_at_ms: i64,
    pub(in crate::v2) heartbeat_at_ms: i64,
    pub(in crate::v2) lease_expires_at_ms: i64,
    pub(in crate::v2) released_at_ms: Option<i64>,
    pub(in crate::v2) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_clock_anchors)]
pub(in crate::v2) struct NewClockAnchorRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) writer_generation: String,
    pub(in crate::v2) wall_time_ms: i64,
    pub(in crate::v2) monotonic_ms: i64,
    pub(in crate::v2) source: String,
    pub(in crate::v2) created_at_ms: i64,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_session_cursors)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
pub(in crate::v2) struct SessionCursorRow {
    pub(in crate::v2) session_id: String,
    pub(in crate::v2) next_commit_seq: i64,
    pub(in crate::v2) writer_generation: Option<String>,
    pub(in crate::v2) updated_at_ms: i64,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_session_cursors)]
pub(in crate::v2) struct NewSessionCursorRow {
    pub(in crate::v2) session_id: String,
    pub(in crate::v2) next_commit_seq: i64,
    pub(in crate::v2) writer_generation: Option<String>,
    pub(in crate::v2) updated_at_ms: i64,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_stream_cursors)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
pub(in crate::v2) struct StreamCursorRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) session_id: String,
    pub(in crate::v2) pane_id: String,
    pub(in crate::v2) stream_id: String,
    pub(in crate::v2) next_event_seq: i64,
    pub(in crate::v2) next_byte_seq: i64,
    pub(in crate::v2) updated_at_ms: i64,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_stream_cursors)]
pub(in crate::v2) struct NewStreamCursorRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) session_id: String,
    pub(in crate::v2) pane_id: String,
    pub(in crate::v2) stream_id: String,
    pub(in crate::v2) next_event_seq: i64,
    pub(in crate::v2) next_byte_seq: i64,
    pub(in crate::v2) updated_at_ms: i64,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_commit_log)]
pub(in crate::v2) struct NewCommitLogRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) session_id: String,
    pub(in crate::v2) commit_seq: i64,
    pub(in crate::v2) commit_kind: String,
    pub(in crate::v2) writer_generation: String,
    pub(in crate::v2) occurred_at_ms: i64,
    pub(in crate::v2) created_at_ms: i64,
    pub(in crate::v2) metadata_json: Option<String>,
}

#[derive(Debug, Clone)]
pub(in crate::v2) struct CommitAllocation {
    pub(in crate::v2) id: String,
    pub(in crate::v2) commit_seq: i64,
}

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

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_capture_receipts)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
pub(in crate::v2) struct CaptureReceiptRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) session_id: String,
    pub(in crate::v2) commit_id: Option<String>,
    pub(in crate::v2) source_kind: String,
    pub(in crate::v2) source_event_id_hash: String,
    pub(in crate::v2) source_payload_hash: String,
    pub(in crate::v2) received_at_ms: i64,
    pub(in crate::v2) created_at_ms: i64,
    pub(in crate::v2) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_capture_receipts)]
pub(in crate::v2) struct NewCaptureReceiptRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) session_id: String,
    pub(in crate::v2) commit_id: Option<String>,
    pub(in crate::v2) source_kind: String,
    pub(in crate::v2) source_event_id_hash: String,
    pub(in crate::v2) source_payload_hash: String,
    pub(in crate::v2) received_at_ms: i64,
    pub(in crate::v2) created_at_ms: i64,
    pub(in crate::v2) metadata_json: Option<String>,
}
