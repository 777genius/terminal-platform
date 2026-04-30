use super::super::*;

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

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_screen_snapshots)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
pub(in crate::v2) struct ScreenSnapshotRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) session_id: String,
    pub(in crate::v2) pane_id: String,
    pub(in crate::v2) commit_id: String,
    pub(in crate::v2) projection_source: String,
    pub(in crate::v2) buffer_kind: String,
    pub(in crate::v2) rows: i32,
    pub(in crate::v2) cols: i32,
    pub(in crate::v2) base_event_seq: i64,
    pub(in crate::v2) high_water_event_seq: i64,
    pub(in crate::v2) high_water_byte_seq: Option<i64>,
    pub(in crate::v2) screen_json: String,
    pub(in crate::v2) parser_version: String,
    pub(in crate::v2) projection_version: String,
    pub(in crate::v2) checksum_algorithm: String,
    pub(in crate::v2) checksum: String,
    pub(in crate::v2) created_at_ms: i64,
    pub(in crate::v2) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_screen_snapshots)]
pub(in crate::v2) struct NewScreenSnapshotRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) session_id: String,
    pub(in crate::v2) pane_id: String,
    pub(in crate::v2) commit_id: String,
    pub(in crate::v2) projection_source: String,
    pub(in crate::v2) buffer_kind: String,
    pub(in crate::v2) rows: i32,
    pub(in crate::v2) cols: i32,
    pub(in crate::v2) base_event_seq: i64,
    pub(in crate::v2) high_water_event_seq: i64,
    pub(in crate::v2) high_water_byte_seq: Option<i64>,
    pub(in crate::v2) screen_json: String,
    pub(in crate::v2) parser_version: String,
    pub(in crate::v2) projection_version: String,
    pub(in crate::v2) checksum_algorithm: String,
    pub(in crate::v2) checksum: String,
    pub(in crate::v2) created_at_ms: i64,
    pub(in crate::v2) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_topology_snapshots)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
pub(in crate::v2) struct TopologySnapshotRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) session_id: String,
    pub(in crate::v2) commit_id: String,
    pub(in crate::v2) high_water_commit_seq: i64,
    pub(in crate::v2) pane_high_water_json: String,
    pub(in crate::v2) topology_json: String,
    pub(in crate::v2) payload_schema_id: Option<String>,
    pub(in crate::v2) checksum_algorithm: String,
    pub(in crate::v2) checksum: String,
    pub(in crate::v2) source: String,
    pub(in crate::v2) created_at_ms: i64,
    pub(in crate::v2) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_topology_snapshots)]
pub(in crate::v2) struct NewTopologySnapshotRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) session_id: String,
    pub(in crate::v2) commit_id: String,
    pub(in crate::v2) high_water_commit_seq: i64,
    pub(in crate::v2) pane_high_water_json: String,
    pub(in crate::v2) topology_json: String,
    pub(in crate::v2) payload_schema_id: Option<String>,
    pub(in crate::v2) checksum_algorithm: String,
    pub(in crate::v2) checksum: String,
    pub(in crate::v2) source: String,
    pub(in crate::v2) created_at_ms: i64,
    pub(in crate::v2) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_restore_drills)]
pub(in crate::v2) struct NewRestoreDrillRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) session_id: String,
    pub(in crate::v2) drill_kind: String,
    pub(in crate::v2) result: String,
    pub(in crate::v2) restore_guarantee_level: String,
    pub(in crate::v2) checked_at_ms: i64,
    pub(in crate::v2) duration_ms: Option<i64>,
    pub(in crate::v2) source_snapshot_id: Option<String>,
    pub(in crate::v2) evidence_json: Option<String>,
    pub(in crate::v2) error: Option<String>,
    pub(in crate::v2) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_integrity_checks)]
pub(in crate::v2) struct NewIntegrityCheckRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) check_kind: String,
    pub(in crate::v2) scope_kind: String,
    pub(in crate::v2) scope_ref: Option<String>,
    pub(in crate::v2) result: String,
    pub(in crate::v2) checked_at_ms: i64,
    pub(in crate::v2) details_json: Option<String>,
    pub(in crate::v2) error: Option<String>,
    pub(in crate::v2) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_data_health_records)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
pub(in crate::v2) struct DataHealthRecordRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) session_id: Option<String>,
    pub(in crate::v2) pane_id: Option<String>,
    pub(in crate::v2) detection_kind: String,
    pub(in crate::v2) severity: String,
    pub(in crate::v2) first_bad_event_seq: Option<i64>,
    pub(in crate::v2) affected_ref: Option<String>,
    pub(in crate::v2) action_state: String,
    pub(in crate::v2) detected_at_ms: i64,
    pub(in crate::v2) resolved_at_ms: Option<i64>,
    pub(in crate::v2) details_json: Option<String>,
    pub(in crate::v2) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_data_health_records)]
pub(in crate::v2) struct NewDataHealthRecordRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) session_id: Option<String>,
    pub(in crate::v2) pane_id: Option<String>,
    pub(in crate::v2) detection_kind: String,
    pub(in crate::v2) severity: String,
    pub(in crate::v2) first_bad_event_seq: Option<i64>,
    pub(in crate::v2) affected_ref: Option<String>,
    pub(in crate::v2) action_state: String,
    pub(in crate::v2) detected_at_ms: i64,
    pub(in crate::v2) resolved_at_ms: Option<i64>,
    pub(in crate::v2) details_json: Option<String>,
    pub(in crate::v2) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_backup_records)]
pub(in crate::v2) struct NewBackupRecordRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) backup_kind: String,
    pub(in crate::v2) state: String,
    pub(in crate::v2) target_ref_hash: Option<String>,
    pub(in crate::v2) manifest_json: Option<String>,
    pub(in crate::v2) checksum_algorithm: Option<String>,
    pub(in crate::v2) checksum: Option<String>,
    pub(in crate::v2) source_db_path_hash: Option<String>,
    pub(in crate::v2) started_at_ms: i64,
    pub(in crate::v2) finished_at_ms: Option<i64>,
    pub(in crate::v2) quick_check_result: Option<String>,
    pub(in crate::v2) error: Option<String>,
    pub(in crate::v2) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_storage_pressure_events)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
pub(in crate::v2) struct StoragePressureEventRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) state: String,
    pub(in crate::v2) db_file_bytes: Option<i64>,
    pub(in crate::v2) wal_file_bytes: Option<i64>,
    pub(in crate::v2) disk_free_bytes: Option<i64>,
    pub(in crate::v2) temp_free_bytes: Option<i64>,
    pub(in crate::v2) quota_bytes: Option<i64>,
    pub(in crate::v2) action_taken: String,
    pub(in crate::v2) reason: Option<String>,
    pub(in crate::v2) created_at_ms: i64,
    pub(in crate::v2) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_storage_pressure_events)]
pub(in crate::v2) struct NewStoragePressureEventRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) state: String,
    pub(in crate::v2) db_file_bytes: Option<i64>,
    pub(in crate::v2) wal_file_bytes: Option<i64>,
    pub(in crate::v2) disk_free_bytes: Option<i64>,
    pub(in crate::v2) temp_free_bytes: Option<i64>,
    pub(in crate::v2) quota_bytes: Option<i64>,
    pub(in crate::v2) action_taken: String,
    pub(in crate::v2) reason: Option<String>,
    pub(in crate::v2) created_at_ms: i64,
    pub(in crate::v2) metadata_json: Option<String>,
}
