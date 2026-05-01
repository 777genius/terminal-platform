use super::super::super::*;

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_command_blocks)]
pub(in crate::v2) struct NewCommandBlockRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) session_id: String,
    pub(in crate::v2) pane_id: String,
    pub(in crate::v2) commit_id: Option<String>,
    pub(in crate::v2) command_text: Option<String>,
    pub(in crate::v2) display_text: Option<String>,
    pub(in crate::v2) redacted_text: Option<String>,
    pub(in crate::v2) command_text_source: String,
    pub(in crate::v2) trust_level: String,
    pub(in crate::v2) state: String,
    pub(in crate::v2) cwd: Option<String>,
    pub(in crate::v2) cwd_source: Option<String>,
    pub(in crate::v2) exit_code: Option<i32>,
    pub(in crate::v2) started_event_seq: Option<i64>,
    pub(in crate::v2) submitted_event_seq: Option<i64>,
    pub(in crate::v2) finished_event_seq: Option<i64>,
    pub(in crate::v2) output_event_seq_low: Option<i64>,
    pub(in crate::v2) output_event_seq_high: Option<i64>,
    pub(in crate::v2) output_byte_low: Option<i64>,
    pub(in crate::v2) output_byte_high: Option<i64>,
    pub(in crate::v2) sensitivity_class: String,
    pub(in crate::v2) created_at_ms: i64,
    pub(in crate::v2) updated_at_ms: i64,
    pub(in crate::v2) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_command_history_entries)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
pub(in crate::v2) struct CommandHistoryEntryRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) session_id: Option<String>,
    pub(in crate::v2) pane_id: Option<String>,
    pub(in crate::v2) command_block_id: Option<String>,
    pub(in crate::v2) scope_kind: String,
    pub(in crate::v2) command_text: Option<String>,
    pub(in crate::v2) display_text: String,
    pub(in crate::v2) redacted_text: Option<String>,
    pub(in crate::v2) command_hash_algorithm: String,
    pub(in crate::v2) command_hash_scope: String,
    pub(in crate::v2) command_hash: String,
    pub(in crate::v2) cwd: Option<String>,
    pub(in crate::v2) shell_kind: Option<String>,
    pub(in crate::v2) trust_level: String,
    pub(in crate::v2) source: String,
    pub(in crate::v2) sensitivity_class: String,
    pub(in crate::v2) redaction_state: String,
    pub(in crate::v2) rerun_policy: String,
    pub(in crate::v2) first_used_at_ms: i64,
    pub(in crate::v2) last_used_at_ms: i64,
    pub(in crate::v2) use_count: i64,
    pub(in crate::v2) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_command_history_entries)]
pub(in crate::v2) struct NewCommandHistoryEntryRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) session_id: Option<String>,
    pub(in crate::v2) pane_id: Option<String>,
    pub(in crate::v2) command_block_id: Option<String>,
    pub(in crate::v2) scope_kind: String,
    pub(in crate::v2) command_text: Option<String>,
    pub(in crate::v2) display_text: String,
    pub(in crate::v2) redacted_text: Option<String>,
    pub(in crate::v2) command_hash_algorithm: String,
    pub(in crate::v2) command_hash_scope: String,
    pub(in crate::v2) command_hash: String,
    pub(in crate::v2) cwd: Option<String>,
    pub(in crate::v2) shell_kind: Option<String>,
    pub(in crate::v2) trust_level: String,
    pub(in crate::v2) source: String,
    pub(in crate::v2) sensitivity_class: String,
    pub(in crate::v2) redaction_state: String,
    pub(in crate::v2) rerun_policy: String,
    pub(in crate::v2) first_used_at_ms: i64,
    pub(in crate::v2) last_used_at_ms: i64,
    pub(in crate::v2) use_count: i64,
    pub(in crate::v2) metadata_json: Option<String>,
}
