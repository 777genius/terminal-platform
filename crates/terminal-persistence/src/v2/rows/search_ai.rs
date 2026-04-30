use super::super::*;

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

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_prompt_injection_findings)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(in crate::v2) struct PromptInjectionFindingRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) package_id: Option<String>,
    pub(in crate::v2) item_id: Option<String>,
    pub(in crate::v2) severity: String,
    pub(in crate::v2) pattern_kind: String,
    pub(in crate::v2) action_state: String,
    pub(in crate::v2) detected_at_ms: i64,
    pub(in crate::v2) evidence_preview: String,
    pub(in crate::v2) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_prompt_injection_findings)]
pub(in crate::v2) struct NewPromptInjectionFindingRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) package_id: Option<String>,
    pub(in crate::v2) item_id: Option<String>,
    pub(in crate::v2) severity: String,
    pub(in crate::v2) pattern_kind: String,
    pub(in crate::v2) action_state: String,
    pub(in crate::v2) detected_at_ms: i64,
    pub(in crate::v2) evidence_preview: String,
    pub(in crate::v2) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_ai_action_approvals)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(in crate::v2) struct AiActionApprovalRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) package_id: Option<String>,
    pub(in crate::v2) action_kind: String,
    pub(in crate::v2) state: String,
    pub(in crate::v2) requester_ref_hash: Option<String>,
    pub(in crate::v2) approver_ref_hash: Option<String>,
    pub(in crate::v2) requested_at_ms: i64,
    pub(in crate::v2) decided_at_ms: Option<i64>,
    pub(in crate::v2) expires_at_ms: Option<i64>,
    pub(in crate::v2) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_ai_action_approvals)]
pub(in crate::v2) struct NewAiActionApprovalRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) package_id: Option<String>,
    pub(in crate::v2) action_kind: String,
    pub(in crate::v2) state: String,
    pub(in crate::v2) requester_ref_hash: Option<String>,
    pub(in crate::v2) approver_ref_hash: Option<String>,
    pub(in crate::v2) requested_at_ms: i64,
    pub(in crate::v2) decided_at_ms: Option<i64>,
    pub(in crate::v2) expires_at_ms: Option<i64>,
    pub(in crate::v2) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_payload_schemas)]
pub(in crate::v2) struct NewPayloadSchemaRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) payload_kind: String,
    pub(in crate::v2) schema_version: String,
    pub(in crate::v2) schema_json: String,
    pub(in crate::v2) schema_hash: String,
    pub(in crate::v2) created_at_ms: i64,
}
