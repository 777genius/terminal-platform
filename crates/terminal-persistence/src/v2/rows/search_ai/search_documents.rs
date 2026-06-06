use super::super::super::*;

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_search_documents)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
pub(in crate::v2) struct SearchDocumentRow {
    pub(in crate::v2) rowid: i32,
    pub(in crate::v2) document_id: String,
    pub(in crate::v2) session_id: String,
    pub(in crate::v2) pane_id: Option<String>,
    pub(in crate::v2) command_block_id: Option<String>,
    pub(in crate::v2) document_kind: String,
    pub(in crate::v2) event_seq_low: Option<i64>,
    pub(in crate::v2) event_seq_high: Option<i64>,
    pub(in crate::v2) byte_low: Option<i64>,
    pub(in crate::v2) byte_high: Option<i64>,
    pub(in crate::v2) redaction_profile_id: Option<String>,
    pub(in crate::v2) redaction_state: String,
    pub(in crate::v2) source_hash_algorithm: String,
    pub(in crate::v2) source_hash: String,
    pub(in crate::v2) text_preview: String,
    pub(in crate::v2) updated_at_ms: i64,
    pub(in crate::v2) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_search_documents)]
pub(in crate::v2) struct NewSearchDocumentRow {
    pub(in crate::v2) document_id: String,
    pub(in crate::v2) session_id: String,
    pub(in crate::v2) pane_id: Option<String>,
    pub(in crate::v2) command_block_id: Option<String>,
    pub(in crate::v2) document_kind: String,
    pub(in crate::v2) event_seq_low: Option<i64>,
    pub(in crate::v2) event_seq_high: Option<i64>,
    pub(in crate::v2) byte_low: Option<i64>,
    pub(in crate::v2) byte_high: Option<i64>,
    pub(in crate::v2) redaction_profile_id: Option<String>,
    pub(in crate::v2) redaction_state: String,
    pub(in crate::v2) source_hash_algorithm: String,
    pub(in crate::v2) source_hash: String,
    pub(in crate::v2) text_preview: String,
    pub(in crate::v2) updated_at_ms: i64,
    pub(in crate::v2) metadata_json: Option<String>,
}
