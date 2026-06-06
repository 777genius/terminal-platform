use super::super::super::*;

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
