use super::*;

#[derive(Debug, Clone, Queryable, Selectable, Insertable)]
#[diesel(table_name = terminal_db_identity)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct TerminalDbIdentityRow {
    pub id: i32,
    pub product: String,
    pub schema_family: String,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
    pub app_version: Option<String>,
    pub diesel_version: Option<String>,
    pub sqlite_version: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_feature_gates)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
pub(super) struct FeatureGateRow {
    pub(super) id: String,
    pub(super) feature_name: String,
    pub(super) state: String,
    pub(super) rollout_scope: String,
    pub(super) reason: Option<String>,
    pub(super) enabled_at_ms: Option<i64>,
    pub(super) disabled_at_ms: Option<i64>,
    pub(super) updated_at_ms: i64,
    pub(super) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_maintenance_runs)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
pub(super) struct MaintenanceRunRow {
    pub(super) id: String,
    pub(super) run_kind: String,
    pub(super) state: String,
    pub(super) selected_policy_id: Option<String>,
    pub(super) started_at_ms: i64,
    pub(super) finished_at_ms: Option<i64>,
    pub(super) summary_json: Option<String>,
    pub(super) error: Option<String>,
    pub(super) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_maintenance_runs)]
pub(super) struct NewMaintenanceRunRow {
    pub(super) id: String,
    pub(super) run_kind: String,
    pub(super) state: String,
    pub(super) selected_policy_id: Option<String>,
    pub(super) started_at_ms: i64,
    pub(super) finished_at_ms: Option<i64>,
    pub(super) summary_json: Option<String>,
    pub(super) error: Option<String>,
    pub(super) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_sessions)]
pub(super) struct NewTerminalSessionRow {
    pub(super) id: String,
    pub(super) route_json: String,
    pub(super) title: Option<String>,
    pub(super) launch_json: Option<String>,
    pub(super) source: String,
    pub(super) durability_profile: String,
    pub(super) retention_policy_id: String,
    pub(super) private_mode: i32,
    pub(super) created_at_ms: i64,
    pub(super) updated_at_ms: i64,
    pub(super) closed_at_ms: Option<i64>,
    pub(super) state: String,
    pub(super) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_panes)]
pub(super) struct NewTerminalPaneRow {
    pub(super) id: String,
    pub(super) session_id: String,
    pub(super) tab_id: Option<String>,
    pub(super) stream_id: String,
    pub(super) title: Option<String>,
    pub(super) rows: i32,
    pub(super) cols: i32,
    pub(super) last_event_seq: i64,
    pub(super) created_at_ms: i64,
    pub(super) closed_at_ms: Option<i64>,
    pub(super) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_backend_capability_reports)]
pub(super) struct NewBackendCapabilityReportRow {
    pub(super) id: String,
    pub(super) session_id: Option<String>,
    pub(super) backend_kind: String,
    pub(super) backend_version: Option<String>,
    pub(super) backend_binary_path_hash: Option<String>,
    pub(super) route_kind: String,
    pub(super) probe_status: String,
    pub(super) capture_strategy: String,
    pub(super) capture_semantics: String,
    pub(super) can_preserve_process_when_live: i32,
    pub(super) can_capture_scrollback: i32,
    pub(super) command_boundary_confidence: String,
    pub(super) evidence_json: Option<String>,
    pub(super) created_at_ms: i64,
    pub(super) expires_at_ms: i64,
    pub(super) stale_reason: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_backend_capability_reports)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
pub(super) struct BackendCapabilityReportRow {
    pub(super) id: String,
    pub(super) session_id: Option<String>,
    pub(super) backend_kind: String,
    pub(super) backend_version: Option<String>,
    pub(super) backend_binary_path_hash: Option<String>,
    pub(super) route_kind: String,
    pub(super) probe_status: String,
    pub(super) capture_strategy: String,
    pub(super) capture_semantics: String,
    pub(super) can_preserve_process_when_live: i32,
    pub(super) can_capture_scrollback: i32,
    pub(super) command_boundary_confidence: String,
    pub(super) evidence_json: Option<String>,
    pub(super) created_at_ms: i64,
    pub(super) expires_at_ms: i64,
    pub(super) stale_reason: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_writer_generations)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
pub(super) struct WriterGenerationRow {
    pub(super) id: String,
    pub(super) process_id: String,
    pub(super) lease_token: String,
    pub(super) state: String,
    pub(super) acquired_at_ms: i64,
    pub(super) heartbeat_at_ms: i64,
    pub(super) lease_expires_at_ms: i64,
    pub(super) released_at_ms: Option<i64>,
    pub(super) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_writer_generations)]
pub(super) struct NewWriterGenerationRow {
    pub(super) id: String,
    pub(super) process_id: String,
    pub(super) lease_token: String,
    pub(super) state: String,
    pub(super) acquired_at_ms: i64,
    pub(super) heartbeat_at_ms: i64,
    pub(super) lease_expires_at_ms: i64,
    pub(super) released_at_ms: Option<i64>,
    pub(super) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_clock_anchors)]
pub(super) struct NewClockAnchorRow {
    pub(super) id: String,
    pub(super) writer_generation: String,
    pub(super) wall_time_ms: i64,
    pub(super) monotonic_ms: i64,
    pub(super) source: String,
    pub(super) created_at_ms: i64,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_session_cursors)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
pub(super) struct SessionCursorRow {
    pub(super) session_id: String,
    pub(super) next_commit_seq: i64,
    pub(super) writer_generation: Option<String>,
    pub(super) updated_at_ms: i64,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_session_cursors)]
pub(super) struct NewSessionCursorRow {
    pub(super) session_id: String,
    pub(super) next_commit_seq: i64,
    pub(super) writer_generation: Option<String>,
    pub(super) updated_at_ms: i64,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_stream_cursors)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
pub(super) struct StreamCursorRow {
    pub(super) id: String,
    pub(super) session_id: String,
    pub(super) pane_id: String,
    pub(super) stream_id: String,
    pub(super) next_event_seq: i64,
    pub(super) next_byte_seq: i64,
    pub(super) updated_at_ms: i64,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_stream_cursors)]
pub(super) struct NewStreamCursorRow {
    pub(super) id: String,
    pub(super) session_id: String,
    pub(super) pane_id: String,
    pub(super) stream_id: String,
    pub(super) next_event_seq: i64,
    pub(super) next_byte_seq: i64,
    pub(super) updated_at_ms: i64,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_commit_log)]
pub(super) struct NewCommitLogRow {
    pub(super) id: String,
    pub(super) session_id: String,
    pub(super) commit_seq: i64,
    pub(super) commit_kind: String,
    pub(super) writer_generation: String,
    pub(super) occurred_at_ms: i64,
    pub(super) created_at_ms: i64,
    pub(super) metadata_json: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) struct CommitAllocation {
    pub(super) id: String,
    pub(super) commit_seq: i64,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_stream_segments)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
pub(super) struct StreamSegmentRow {
    pub(super) id: String,
    pub(super) session_id: String,
    pub(super) pane_id: String,
    pub(super) commit_id: String,
    pub(super) stream_id: String,
    pub(super) event_seq_low: i64,
    pub(super) event_seq_high: i64,
    pub(super) byte_low: i64,
    pub(super) byte_high: i64,
    pub(super) payload: Vec<u8>,
    pub(super) payload_len: i64,
    pub(super) stored_byte_len: i64,
    pub(super) uncompressed_byte_len: Option<i64>,
    pub(super) checksum_algorithm: String,
    pub(super) checksum: String,
    pub(super) compression: String,
    pub(super) capture_semantics: String,
    pub(super) encryption_state: String,
    pub(super) key_ref: Option<String>,
    pub(super) created_at_ms: i64,
    pub(super) writer_generation: String,
    pub(super) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_stream_segments)]
pub(super) struct NewStreamSegmentRow {
    pub(super) id: String,
    pub(super) session_id: String,
    pub(super) pane_id: String,
    pub(super) commit_id: String,
    pub(super) stream_id: String,
    pub(super) event_seq_low: i64,
    pub(super) event_seq_high: i64,
    pub(super) byte_low: i64,
    pub(super) byte_high: i64,
    pub(super) payload: Vec<u8>,
    pub(super) payload_len: i64,
    pub(super) stored_byte_len: i64,
    pub(super) uncompressed_byte_len: Option<i64>,
    pub(super) checksum_algorithm: String,
    pub(super) checksum: String,
    pub(super) compression: String,
    pub(super) capture_semantics: String,
    pub(super) encryption_state: String,
    pub(super) key_ref: Option<String>,
    pub(super) created_at_ms: i64,
    pub(super) writer_generation: String,
    pub(super) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_journal_events)]
pub(super) struct NewJournalEventRow {
    pub(super) id: String,
    pub(super) session_id: String,
    pub(super) pane_id: Option<String>,
    pub(super) commit_id: String,
    pub(super) stream_id: String,
    pub(super) event_scope_kind: String,
    pub(super) event_scope_id: String,
    pub(super) event_seq: i64,
    pub(super) event_type: String,
    pub(super) byte_low: Option<i64>,
    pub(super) byte_high: Option<i64>,
    pub(super) payload_json: Option<String>,
    pub(super) payload_schema_id: Option<String>,
    pub(super) source_event_id_hash: Option<String>,
    pub(super) occurred_at_ms: i64,
    pub(super) created_at_ms: i64,
    pub(super) capture_semantics: String,
    pub(super) trust_level: String,
    pub(super) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_capture_receipts)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
pub(super) struct CaptureReceiptRow {
    pub(super) id: String,
    pub(super) session_id: String,
    pub(super) commit_id: Option<String>,
    pub(super) source_kind: String,
    pub(super) source_event_id_hash: String,
    pub(super) source_payload_hash: String,
    pub(super) received_at_ms: i64,
    pub(super) created_at_ms: i64,
    pub(super) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_capture_receipts)]
pub(super) struct NewCaptureReceiptRow {
    pub(super) id: String,
    pub(super) session_id: String,
    pub(super) commit_id: Option<String>,
    pub(super) source_kind: String,
    pub(super) source_event_id_hash: String,
    pub(super) source_payload_hash: String,
    pub(super) received_at_ms: i64,
    pub(super) created_at_ms: i64,
    pub(super) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_clients)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
pub(super) struct DeliveryClientRow {
    pub(super) id: String,
    pub(super) client_kind: String,
    pub(super) install_ref_hash: Option<String>,
    pub(super) browser_profile_ref_hash: Option<String>,
    pub(super) user_agent_hash: Option<String>,
    pub(super) created_at_ms: i64,
    pub(super) last_seen_at_ms: i64,
    pub(super) trust_state: String,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_clients)]
pub(super) struct NewDeliveryClientRow {
    pub(super) id: String,
    pub(super) client_kind: String,
    pub(super) install_ref_hash: Option<String>,
    pub(super) browser_profile_ref_hash: Option<String>,
    pub(super) user_agent_hash: Option<String>,
    pub(super) created_at_ms: i64,
    pub(super) last_seen_at_ms: i64,
    pub(super) trust_state: String,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_delivery_offsets)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
pub(super) struct DeliveryOffsetRow {
    pub(super) id: String,
    pub(super) client_id: String,
    pub(super) session_id: String,
    pub(super) pane_id: Option<String>,
    pub(super) stream_id: String,
    pub(super) last_sent_event_seq: i64,
    pub(super) last_acked_event_seq: i64,
    pub(super) last_persisted_event_seq: i64,
    pub(super) replay_from_event_seq: Option<i64>,
    pub(super) gap_state: String,
    pub(super) updated_at_ms: i64,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_delivery_offsets)]
pub(super) struct NewDeliveryOffsetRow {
    pub(super) id: String,
    pub(super) client_id: String,
    pub(super) session_id: String,
    pub(super) pane_id: Option<String>,
    pub(super) stream_id: String,
    pub(super) last_sent_event_seq: i64,
    pub(super) last_acked_event_seq: i64,
    pub(super) last_persisted_event_seq: i64,
    pub(super) replay_from_event_seq: Option<i64>,
    pub(super) gap_state: String,
    pub(super) updated_at_ms: i64,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_outbox_messages)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
pub(super) struct OutboxMessageRow {
    pub(super) id: String,
    pub(super) message_kind: String,
    pub(super) dedupe_key: Option<String>,
    pub(super) state: String,
    pub(super) payload_json: String,
    pub(super) attempts: i64,
    pub(super) max_attempts: i64,
    pub(super) claimed_by: Option<String>,
    pub(super) lease_token: Option<String>,
    pub(super) claimed_until_ms: Option<i64>,
    pub(super) next_run_at_ms: i64,
    pub(super) last_error: Option<String>,
    pub(super) created_at_ms: i64,
    pub(super) updated_at_ms: i64,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_outbox_messages)]
pub(super) struct NewOutboxMessageRow {
    pub(super) id: String,
    pub(super) message_kind: String,
    pub(super) dedupe_key: Option<String>,
    pub(super) state: String,
    pub(super) payload_json: String,
    pub(super) attempts: i64,
    pub(super) max_attempts: i64,
    pub(super) claimed_by: Option<String>,
    pub(super) lease_token: Option<String>,
    pub(super) claimed_until_ms: Option<i64>,
    pub(super) next_run_at_ms: i64,
    pub(super) last_error: Option<String>,
    pub(super) created_at_ms: i64,
    pub(super) updated_at_ms: i64,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_history_gaps)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
pub(super) struct HistoryGapRow {
    pub(super) id: String,
    pub(super) session_id: String,
    pub(super) pane_id: Option<String>,
    pub(super) stream_id: String,
    pub(super) gap_kind: String,
    pub(super) event_seq_low: Option<i64>,
    pub(super) event_seq_high: Option<i64>,
    pub(super) byte_low: Option<i64>,
    pub(super) byte_high: Option<i64>,
    pub(super) estimated_dropped_bytes: Option<i64>,
    pub(super) estimated_dropped_events: Option<i64>,
    pub(super) reason: String,
    pub(super) writer_generation: Option<String>,
    pub(super) opened_at_ms: i64,
    pub(super) closed_at_ms: Option<i64>,
    pub(super) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_history_gaps)]
pub(super) struct NewHistoryGapRow {
    pub(super) id: String,
    pub(super) session_id: String,
    pub(super) pane_id: Option<String>,
    pub(super) stream_id: String,
    pub(super) gap_kind: String,
    pub(super) event_seq_low: Option<i64>,
    pub(super) event_seq_high: Option<i64>,
    pub(super) byte_low: Option<i64>,
    pub(super) byte_high: Option<i64>,
    pub(super) estimated_dropped_bytes: Option<i64>,
    pub(super) estimated_dropped_events: Option<i64>,
    pub(super) reason: String,
    pub(super) writer_generation: Option<String>,
    pub(super) opened_at_ms: i64,
    pub(super) closed_at_ms: Option<i64>,
    pub(super) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_command_blocks)]
pub(super) struct NewCommandBlockRow {
    pub(super) id: String,
    pub(super) session_id: String,
    pub(super) pane_id: String,
    pub(super) commit_id: Option<String>,
    pub(super) command_text: Option<String>,
    pub(super) display_text: Option<String>,
    pub(super) redacted_text: Option<String>,
    pub(super) command_text_source: String,
    pub(super) trust_level: String,
    pub(super) state: String,
    pub(super) cwd: Option<String>,
    pub(super) cwd_source: Option<String>,
    pub(super) exit_code: Option<i32>,
    pub(super) started_event_seq: Option<i64>,
    pub(super) submitted_event_seq: Option<i64>,
    pub(super) finished_event_seq: Option<i64>,
    pub(super) output_event_seq_low: Option<i64>,
    pub(super) output_event_seq_high: Option<i64>,
    pub(super) output_byte_low: Option<i64>,
    pub(super) output_byte_high: Option<i64>,
    pub(super) sensitivity_class: String,
    pub(super) created_at_ms: i64,
    pub(super) updated_at_ms: i64,
    pub(super) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_command_history_entries)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
pub(super) struct CommandHistoryEntryRow {
    pub(super) id: String,
    pub(super) session_id: Option<String>,
    pub(super) pane_id: Option<String>,
    pub(super) command_block_id: Option<String>,
    pub(super) scope_kind: String,
    pub(super) command_text: Option<String>,
    pub(super) display_text: String,
    pub(super) redacted_text: Option<String>,
    pub(super) command_hash_algorithm: String,
    pub(super) command_hash_scope: String,
    pub(super) command_hash: String,
    pub(super) cwd: Option<String>,
    pub(super) shell_kind: Option<String>,
    pub(super) trust_level: String,
    pub(super) source: String,
    pub(super) sensitivity_class: String,
    pub(super) redaction_state: String,
    pub(super) rerun_policy: String,
    pub(super) first_used_at_ms: i64,
    pub(super) last_used_at_ms: i64,
    pub(super) use_count: i64,
    pub(super) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_command_history_entries)]
pub(super) struct NewCommandHistoryEntryRow {
    pub(super) id: String,
    pub(super) session_id: Option<String>,
    pub(super) pane_id: Option<String>,
    pub(super) command_block_id: Option<String>,
    pub(super) scope_kind: String,
    pub(super) command_text: Option<String>,
    pub(super) display_text: String,
    pub(super) redacted_text: Option<String>,
    pub(super) command_hash_algorithm: String,
    pub(super) command_hash_scope: String,
    pub(super) command_hash: String,
    pub(super) cwd: Option<String>,
    pub(super) shell_kind: Option<String>,
    pub(super) trust_level: String,
    pub(super) source: String,
    pub(super) sensitivity_class: String,
    pub(super) redaction_state: String,
    pub(super) rerun_policy: String,
    pub(super) first_used_at_ms: i64,
    pub(super) last_used_at_ms: i64,
    pub(super) use_count: i64,
    pub(super) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_screen_snapshots)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
pub(super) struct ScreenSnapshotRow {
    pub(super) id: String,
    pub(super) session_id: String,
    pub(super) pane_id: String,
    pub(super) commit_id: String,
    pub(super) projection_source: String,
    pub(super) buffer_kind: String,
    pub(super) rows: i32,
    pub(super) cols: i32,
    pub(super) base_event_seq: i64,
    pub(super) high_water_event_seq: i64,
    pub(super) high_water_byte_seq: Option<i64>,
    pub(super) screen_json: String,
    pub(super) parser_version: String,
    pub(super) projection_version: String,
    pub(super) checksum_algorithm: String,
    pub(super) checksum: String,
    pub(super) created_at_ms: i64,
    pub(super) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_screen_snapshots)]
pub(super) struct NewScreenSnapshotRow {
    pub(super) id: String,
    pub(super) session_id: String,
    pub(super) pane_id: String,
    pub(super) commit_id: String,
    pub(super) projection_source: String,
    pub(super) buffer_kind: String,
    pub(super) rows: i32,
    pub(super) cols: i32,
    pub(super) base_event_seq: i64,
    pub(super) high_water_event_seq: i64,
    pub(super) high_water_byte_seq: Option<i64>,
    pub(super) screen_json: String,
    pub(super) parser_version: String,
    pub(super) projection_version: String,
    pub(super) checksum_algorithm: String,
    pub(super) checksum: String,
    pub(super) created_at_ms: i64,
    pub(super) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_topology_snapshots)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
pub(super) struct TopologySnapshotRow {
    pub(super) id: String,
    pub(super) session_id: String,
    pub(super) commit_id: String,
    pub(super) high_water_commit_seq: i64,
    pub(super) pane_high_water_json: String,
    pub(super) topology_json: String,
    pub(super) payload_schema_id: Option<String>,
    pub(super) checksum_algorithm: String,
    pub(super) checksum: String,
    pub(super) source: String,
    pub(super) created_at_ms: i64,
    pub(super) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_topology_snapshots)]
pub(super) struct NewTopologySnapshotRow {
    pub(super) id: String,
    pub(super) session_id: String,
    pub(super) commit_id: String,
    pub(super) high_water_commit_seq: i64,
    pub(super) pane_high_water_json: String,
    pub(super) topology_json: String,
    pub(super) payload_schema_id: Option<String>,
    pub(super) checksum_algorithm: String,
    pub(super) checksum: String,
    pub(super) source: String,
    pub(super) created_at_ms: i64,
    pub(super) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_restore_drills)]
pub(super) struct NewRestoreDrillRow {
    pub(super) id: String,
    pub(super) session_id: String,
    pub(super) drill_kind: String,
    pub(super) result: String,
    pub(super) restore_guarantee_level: String,
    pub(super) checked_at_ms: i64,
    pub(super) duration_ms: Option<i64>,
    pub(super) source_snapshot_id: Option<String>,
    pub(super) evidence_json: Option<String>,
    pub(super) error: Option<String>,
    pub(super) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_integrity_checks)]
pub(super) struct NewIntegrityCheckRow {
    pub(super) id: String,
    pub(super) check_kind: String,
    pub(super) scope_kind: String,
    pub(super) scope_ref: Option<String>,
    pub(super) result: String,
    pub(super) checked_at_ms: i64,
    pub(super) details_json: Option<String>,
    pub(super) error: Option<String>,
    pub(super) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_data_health_records)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
pub(super) struct DataHealthRecordRow {
    pub(super) id: String,
    pub(super) session_id: Option<String>,
    pub(super) pane_id: Option<String>,
    pub(super) detection_kind: String,
    pub(super) severity: String,
    pub(super) first_bad_event_seq: Option<i64>,
    pub(super) affected_ref: Option<String>,
    pub(super) action_state: String,
    pub(super) detected_at_ms: i64,
    pub(super) resolved_at_ms: Option<i64>,
    pub(super) details_json: Option<String>,
    pub(super) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_data_health_records)]
pub(super) struct NewDataHealthRecordRow {
    pub(super) id: String,
    pub(super) session_id: Option<String>,
    pub(super) pane_id: Option<String>,
    pub(super) detection_kind: String,
    pub(super) severity: String,
    pub(super) first_bad_event_seq: Option<i64>,
    pub(super) affected_ref: Option<String>,
    pub(super) action_state: String,
    pub(super) detected_at_ms: i64,
    pub(super) resolved_at_ms: Option<i64>,
    pub(super) details_json: Option<String>,
    pub(super) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_backup_records)]
pub(super) struct NewBackupRecordRow {
    pub(super) id: String,
    pub(super) backup_kind: String,
    pub(super) state: String,
    pub(super) target_ref_hash: Option<String>,
    pub(super) manifest_json: Option<String>,
    pub(super) checksum_algorithm: Option<String>,
    pub(super) checksum: Option<String>,
    pub(super) source_db_path_hash: Option<String>,
    pub(super) started_at_ms: i64,
    pub(super) finished_at_ms: Option<i64>,
    pub(super) quick_check_result: Option<String>,
    pub(super) error: Option<String>,
    pub(super) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_storage_pressure_events)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
pub(super) struct StoragePressureEventRow {
    pub(super) id: String,
    pub(super) state: String,
    pub(super) db_file_bytes: Option<i64>,
    pub(super) wal_file_bytes: Option<i64>,
    pub(super) disk_free_bytes: Option<i64>,
    pub(super) temp_free_bytes: Option<i64>,
    pub(super) quota_bytes: Option<i64>,
    pub(super) action_taken: String,
    pub(super) reason: Option<String>,
    pub(super) created_at_ms: i64,
    pub(super) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_storage_pressure_events)]
pub(super) struct NewStoragePressureEventRow {
    pub(super) id: String,
    pub(super) state: String,
    pub(super) db_file_bytes: Option<i64>,
    pub(super) wal_file_bytes: Option<i64>,
    pub(super) disk_free_bytes: Option<i64>,
    pub(super) temp_free_bytes: Option<i64>,
    pub(super) quota_bytes: Option<i64>,
    pub(super) action_taken: String,
    pub(super) reason: Option<String>,
    pub(super) created_at_ms: i64,
    pub(super) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_delete_requests)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
pub(super) struct DeleteRequestRow {
    pub(super) id: String,
    pub(super) session_id: Option<String>,
    pub(super) request_kind: String,
    pub(super) state: String,
    pub(super) policy_id: Option<String>,
    pub(super) requested_at_ms: i64,
    pub(super) approved_at_ms: Option<i64>,
    pub(super) completed_at_ms: Option<i64>,
    pub(super) requester_ref_hash: Option<String>,
    pub(super) reason: Option<String>,
    pub(super) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_delete_requests)]
pub(super) struct NewDeleteRequestRow {
    pub(super) id: String,
    pub(super) session_id: Option<String>,
    pub(super) request_kind: String,
    pub(super) state: String,
    pub(super) policy_id: Option<String>,
    pub(super) requested_at_ms: i64,
    pub(super) approved_at_ms: Option<i64>,
    pub(super) completed_at_ms: Option<i64>,
    pub(super) requester_ref_hash: Option<String>,
    pub(super) reason: Option<String>,
    pub(super) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_deletion_tombstones)]
pub(super) struct NewDeletionTombstoneRow {
    pub(super) id: String,
    pub(super) delete_request_id: Option<String>,
    pub(super) session_id: Option<String>,
    pub(super) deleted_scope: String,
    pub(super) policy_id: Option<String>,
    pub(super) deleted_at_ms: i64,
    pub(super) evidence_json: Option<String>,
    pub(super) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_export_requests)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(super) struct ExportRequestRow {
    pub(super) id: String,
    pub(super) session_id: Option<String>,
    pub(super) export_kind: String,
    pub(super) state: String,
    pub(super) redaction_profile_id: Option<String>,
    pub(super) include_raw: i32,
    pub(super) approved_at_ms: Option<i64>,
    pub(super) requested_at_ms: i64,
    pub(super) completed_at_ms: Option<i64>,
    pub(super) manifest_json: Option<String>,
    pub(super) output_ref_hash: Option<String>,
    pub(super) error: Option<String>,
    pub(super) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_export_requests)]
pub(super) struct NewExportRequestRow {
    pub(super) id: String,
    pub(super) session_id: Option<String>,
    pub(super) export_kind: String,
    pub(super) state: String,
    pub(super) redaction_profile_id: Option<String>,
    pub(super) include_raw: i32,
    pub(super) approved_at_ms: Option<i64>,
    pub(super) requested_at_ms: i64,
    pub(super) completed_at_ms: Option<i64>,
    pub(super) manifest_json: Option<String>,
    pub(super) output_ref_hash: Option<String>,
    pub(super) error: Option<String>,
    pub(super) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_support_bundles)]
pub(super) struct NewSupportBundleRow {
    pub(super) id: String,
    pub(super) scope_json: String,
    pub(super) state: String,
    pub(super) redaction_profile_id: Option<String>,
    pub(super) include_raw: i32,
    pub(super) requested_at_ms: i64,
    pub(super) completed_at_ms: Option<i64>,
    pub(super) manifest_json: Option<String>,
    pub(super) output_ref_hash: Option<String>,
    pub(super) error: Option<String>,
    pub(super) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_support_bundles)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(super) struct SupportBundleRow {
    pub(super) id: String,
    pub(super) scope_json: String,
    pub(super) state: String,
    pub(super) redaction_profile_id: Option<String>,
    pub(super) include_raw: i32,
    pub(super) requested_at_ms: i64,
    pub(super) completed_at_ms: Option<i64>,
    pub(super) manifest_json: Option<String>,
    pub(super) output_ref_hash: Option<String>,
    pub(super) error: Option<String>,
    pub(super) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_crypto_keys)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
pub(super) struct CryptoKeyRow {
    pub(super) id: String,
    pub(super) key_kind: String,
    pub(super) key_ref: String,
    pub(super) protection_kind: String,
    pub(super) state: String,
    pub(super) created_at_ms: i64,
    pub(super) rotated_at_ms: Option<i64>,
    pub(super) destroyed_at_ms: Option<i64>,
    pub(super) capability_report_json: Option<String>,
    pub(super) error_json: Option<String>,
    pub(super) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_crypto_keys)]
pub(super) struct NewCryptoKeyRow {
    pub(super) id: String,
    pub(super) key_kind: String,
    pub(super) key_ref: String,
    pub(super) protection_kind: String,
    pub(super) state: String,
    pub(super) created_at_ms: i64,
    pub(super) rotated_at_ms: Option<i64>,
    pub(super) destroyed_at_ms: Option<i64>,
    pub(super) capability_report_json: Option<String>,
    pub(super) error_json: Option<String>,
    pub(super) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_crypto_key_events)]
pub(super) struct NewCryptoKeyEventRow {
    pub(super) id: String,
    pub(super) key_id: Option<String>,
    pub(super) event_kind: String,
    pub(super) actor: String,
    pub(super) occurred_at_ms: i64,
    pub(super) status: String,
    pub(super) error_json: Option<String>,
    pub(super) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_external_artifacts)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(super) struct ExternalArtifactRow {
    pub(super) id: String,
    pub(super) artifact_kind: String,
    pub(super) artifact_ref_hash: String,
    pub(super) state: String,
    pub(super) encryption_state: String,
    pub(super) key_ref: Option<String>,
    pub(super) checksum_algorithm: Option<String>,
    pub(super) checksum: Option<String>,
    pub(super) size_bytes: Option<i64>,
    pub(super) created_at_ms: i64,
    pub(super) verified_at_ms: Option<i64>,
    pub(super) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_external_artifacts)]
pub(super) struct NewExternalArtifactRow {
    pub(super) id: String,
    pub(super) artifact_kind: String,
    pub(super) artifact_ref_hash: String,
    pub(super) state: String,
    pub(super) encryption_state: String,
    pub(super) key_ref: Option<String>,
    pub(super) checksum_algorithm: Option<String>,
    pub(super) checksum: Option<String>,
    pub(super) size_bytes: Option<i64>,
    pub(super) created_at_ms: i64,
    pub(super) verified_at_ms: Option<i64>,
    pub(super) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_search_documents)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
pub(super) struct SearchDocumentRow {
    pub(super) rowid: i32,
    pub(super) document_id: String,
    pub(super) session_id: String,
    pub(super) pane_id: Option<String>,
    pub(super) command_block_id: Option<String>,
    pub(super) document_kind: String,
    pub(super) event_seq_low: Option<i64>,
    pub(super) event_seq_high: Option<i64>,
    pub(super) byte_low: Option<i64>,
    pub(super) byte_high: Option<i64>,
    pub(super) redaction_profile_id: Option<String>,
    pub(super) redaction_state: String,
    pub(super) source_hash_algorithm: String,
    pub(super) source_hash: String,
    pub(super) text_preview: String,
    pub(super) updated_at_ms: i64,
    pub(super) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_search_documents)]
pub(super) struct NewSearchDocumentRow {
    pub(super) document_id: String,
    pub(super) session_id: String,
    pub(super) pane_id: Option<String>,
    pub(super) command_block_id: Option<String>,
    pub(super) document_kind: String,
    pub(super) event_seq_low: Option<i64>,
    pub(super) event_seq_high: Option<i64>,
    pub(super) byte_low: Option<i64>,
    pub(super) byte_high: Option<i64>,
    pub(super) redaction_profile_id: Option<String>,
    pub(super) redaction_state: String,
    pub(super) source_hash_algorithm: String,
    pub(super) source_hash: String,
    pub(super) text_preview: String,
    pub(super) updated_at_ms: i64,
    pub(super) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_ai_context_packages)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(super) struct AiContextPackageRow {
    pub(super) id: String,
    pub(super) session_id: Option<String>,
    pub(super) pane_id: Option<String>,
    pub(super) state: String,
    pub(super) redaction_profile_id: Option<String>,
    pub(super) include_raw: i32,
    pub(super) requested_at_ms: i64,
    pub(super) built_at_ms: Option<i64>,
    pub(super) item_count: i64,
    pub(super) manifest_json: Option<String>,
    pub(super) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_ai_context_packages)]
pub(super) struct NewAiContextPackageRow {
    pub(super) id: String,
    pub(super) session_id: Option<String>,
    pub(super) pane_id: Option<String>,
    pub(super) state: String,
    pub(super) redaction_profile_id: Option<String>,
    pub(super) include_raw: i32,
    pub(super) requested_at_ms: i64,
    pub(super) built_at_ms: Option<i64>,
    pub(super) item_count: i64,
    pub(super) manifest_json: Option<String>,
    pub(super) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_ai_context_items)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(super) struct AiContextItemRow {
    pub(super) id: String,
    pub(super) package_id: String,
    pub(super) source_kind: String,
    pub(super) source_ref: Option<String>,
    pub(super) session_id: Option<String>,
    pub(super) pane_id: Option<String>,
    pub(super) command_block_id: Option<String>,
    pub(super) event_seq_low: Option<i64>,
    pub(super) event_seq_high: Option<i64>,
    pub(super) byte_low: Option<i64>,
    pub(super) byte_high: Option<i64>,
    pub(super) redaction_state: String,
    pub(super) data_only: i32,
    pub(super) content_preview: String,
    pub(super) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_ai_context_items)]
pub(super) struct NewAiContextItemRow {
    pub(super) id: String,
    pub(super) package_id: String,
    pub(super) source_kind: String,
    pub(super) source_ref: Option<String>,
    pub(super) session_id: Option<String>,
    pub(super) pane_id: Option<String>,
    pub(super) command_block_id: Option<String>,
    pub(super) event_seq_low: Option<i64>,
    pub(super) event_seq_high: Option<i64>,
    pub(super) byte_low: Option<i64>,
    pub(super) byte_high: Option<i64>,
    pub(super) redaction_state: String,
    pub(super) data_only: i32,
    pub(super) content_preview: String,
    pub(super) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_prompt_injection_findings)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(super) struct PromptInjectionFindingRow {
    pub(super) id: String,
    pub(super) package_id: Option<String>,
    pub(super) item_id: Option<String>,
    pub(super) severity: String,
    pub(super) pattern_kind: String,
    pub(super) action_state: String,
    pub(super) detected_at_ms: i64,
    pub(super) evidence_preview: String,
    pub(super) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_prompt_injection_findings)]
pub(super) struct NewPromptInjectionFindingRow {
    pub(super) id: String,
    pub(super) package_id: Option<String>,
    pub(super) item_id: Option<String>,
    pub(super) severity: String,
    pub(super) pattern_kind: String,
    pub(super) action_state: String,
    pub(super) detected_at_ms: i64,
    pub(super) evidence_preview: String,
    pub(super) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_ai_action_approvals)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(super) struct AiActionApprovalRow {
    pub(super) id: String,
    pub(super) package_id: Option<String>,
    pub(super) action_kind: String,
    pub(super) state: String,
    pub(super) requester_ref_hash: Option<String>,
    pub(super) approver_ref_hash: Option<String>,
    pub(super) requested_at_ms: i64,
    pub(super) decided_at_ms: Option<i64>,
    pub(super) expires_at_ms: Option<i64>,
    pub(super) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_ai_action_approvals)]
pub(super) struct NewAiActionApprovalRow {
    pub(super) id: String,
    pub(super) package_id: Option<String>,
    pub(super) action_kind: String,
    pub(super) state: String,
    pub(super) requester_ref_hash: Option<String>,
    pub(super) approver_ref_hash: Option<String>,
    pub(super) requested_at_ms: i64,
    pub(super) decided_at_ms: Option<i64>,
    pub(super) expires_at_ms: Option<i64>,
    pub(super) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_payload_schemas)]
pub(super) struct NewPayloadSchemaRow {
    pub(super) id: String,
    pub(super) payload_kind: String,
    pub(super) schema_version: String,
    pub(super) schema_json: String,
    pub(super) schema_hash: String,
    pub(super) created_at_ms: i64,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_db_identity)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
pub(super) struct DbIdentityProbeRow {
    pub(super) id: i32,
}

#[derive(Debug, QueryableByName)]
pub(super) struct QuickCheckRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub(super) quick_check: String,
}

#[derive(Debug, QueryableByName, Serialize)]
pub(super) struct WalCheckpointRow {
    #[diesel(sql_type = diesel::sql_types::Integer)]
    pub(super) busy: i32,
    #[diesel(sql_type = diesel::sql_types::Integer)]
    pub(super) log: i32,
    #[diesel(sql_type = diesel::sql_types::Integer)]
    pub(super) checkpointed: i32,
}

#[derive(Debug, QueryableByName, Serialize)]
pub(super) struct ForeignKeyCheckRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub(super) table_name: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::BigInt>)]
    pub(super) rowid: Option<i64>,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub(super) parent: String,
    #[diesel(sql_type = diesel::sql_types::Integer)]
    pub(super) fkid: i32,
}

#[derive(Debug, Clone)]
pub(super) struct HistoryValidation {
    pub(super) journal_events_checked: usize,
    pub(super) stream_segments_checked: usize,
    pub(super) screen_snapshots_checked: usize,
    pub(super) topology_snapshots_checked: usize,
    pub(super) failures: Vec<String>,
}

impl HistoryValidation {
    pub(super) fn has_failures(&self) -> bool {
        !self.failures.is_empty()
    }

    pub(super) fn failure_count(&self) -> usize {
        self.failures.len()
    }

    pub(super) fn checksum_failure_count(&self) -> usize {
        self.failures.iter().filter(|failure| failure.contains("checksum mismatch")).count()
    }

    pub(super) fn summary(&self) -> String {
        if self.failures.is_empty() {
            "history validation passed".to_string()
        } else {
            self.failures.join("; ")
        }
    }

    pub(super) fn to_json(&self) -> Value {
        serde_json::json!({
            "journal_events_checked": self.journal_events_checked,
            "stream_segments_checked": self.stream_segments_checked,
            "screen_snapshots_checked": self.screen_snapshots_checked,
            "topology_snapshots_checked": self.topology_snapshots_checked,
            "failures": self.failures,
        })
    }

    pub(super) fn to_restore_evidence(&self) -> Vec<RestoreEvidence> {
        vec![
            RestoreEvidence {
                kind: "journal_events_checked".to_string(),
                value: self.journal_events_checked.to_string(),
            },
            RestoreEvidence {
                kind: "stream_segments_checked".to_string(),
                value: self.stream_segments_checked.to_string(),
            },
            RestoreEvidence {
                kind: "screen_snapshots_checked".to_string(),
                value: self.screen_snapshots_checked.to_string(),
            },
            RestoreEvidence {
                kind: "topology_snapshots_checked".to_string(),
                value: self.topology_snapshots_checked.to_string(),
            },
            RestoreEvidence {
                kind: "history_validation_failures".to_string(),
                value: self.failures.len().to_string(),
            },
        ]
    }
}
