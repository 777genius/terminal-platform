use super::super::super::*;

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_ai_context_packages)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(in crate::v2) struct AiContextPackageRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) session_id: Option<String>,
    pub(in crate::v2) pane_id: Option<String>,
    pub(in crate::v2) state: String,
    pub(in crate::v2) redaction_profile_id: Option<String>,
    pub(in crate::v2) include_raw: i32,
    pub(in crate::v2) requested_at_ms: i64,
    pub(in crate::v2) built_at_ms: Option<i64>,
    pub(in crate::v2) item_count: i64,
    pub(in crate::v2) manifest_json: Option<String>,
    pub(in crate::v2) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_ai_context_packages)]
pub(in crate::v2) struct NewAiContextPackageRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) session_id: Option<String>,
    pub(in crate::v2) pane_id: Option<String>,
    pub(in crate::v2) state: String,
    pub(in crate::v2) redaction_profile_id: Option<String>,
    pub(in crate::v2) include_raw: i32,
    pub(in crate::v2) requested_at_ms: i64,
    pub(in crate::v2) built_at_ms: Option<i64>,
    pub(in crate::v2) item_count: i64,
    pub(in crate::v2) manifest_json: Option<String>,
    pub(in crate::v2) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_ai_context_items)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(in crate::v2) struct AiContextItemRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) package_id: String,
    pub(in crate::v2) source_kind: String,
    pub(in crate::v2) source_ref: Option<String>,
    pub(in crate::v2) session_id: Option<String>,
    pub(in crate::v2) pane_id: Option<String>,
    pub(in crate::v2) command_block_id: Option<String>,
    pub(in crate::v2) event_seq_low: Option<i64>,
    pub(in crate::v2) event_seq_high: Option<i64>,
    pub(in crate::v2) byte_low: Option<i64>,
    pub(in crate::v2) byte_high: Option<i64>,
    pub(in crate::v2) redaction_state: String,
    pub(in crate::v2) data_only: i32,
    pub(in crate::v2) content_preview: String,
    pub(in crate::v2) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_ai_context_items)]
pub(in crate::v2) struct NewAiContextItemRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) package_id: String,
    pub(in crate::v2) source_kind: String,
    pub(in crate::v2) source_ref: Option<String>,
    pub(in crate::v2) session_id: Option<String>,
    pub(in crate::v2) pane_id: Option<String>,
    pub(in crate::v2) command_block_id: Option<String>,
    pub(in crate::v2) event_seq_low: Option<i64>,
    pub(in crate::v2) event_seq_high: Option<i64>,
    pub(in crate::v2) byte_low: Option<i64>,
    pub(in crate::v2) byte_high: Option<i64>,
    pub(in crate::v2) redaction_state: String,
    pub(in crate::v2) data_only: i32,
    pub(in crate::v2) content_preview: String,
    pub(in crate::v2) metadata_json: Option<String>,
}
