use super::super::*;

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
pub(in crate::v2) struct FeatureGateRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) feature_name: String,
    pub(in crate::v2) state: String,
    pub(in crate::v2) rollout_scope: String,
    pub(in crate::v2) reason: Option<String>,
    pub(in crate::v2) enabled_at_ms: Option<i64>,
    pub(in crate::v2) disabled_at_ms: Option<i64>,
    pub(in crate::v2) updated_at_ms: i64,
    pub(in crate::v2) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_maintenance_runs)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
pub(in crate::v2) struct MaintenanceRunRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) run_kind: String,
    pub(in crate::v2) state: String,
    pub(in crate::v2) selected_policy_id: Option<String>,
    pub(in crate::v2) started_at_ms: i64,
    pub(in crate::v2) finished_at_ms: Option<i64>,
    pub(in crate::v2) summary_json: Option<String>,
    pub(in crate::v2) error: Option<String>,
    pub(in crate::v2) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_maintenance_runs)]
pub(in crate::v2) struct NewMaintenanceRunRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) run_kind: String,
    pub(in crate::v2) state: String,
    pub(in crate::v2) selected_policy_id: Option<String>,
    pub(in crate::v2) started_at_ms: i64,
    pub(in crate::v2) finished_at_ms: Option<i64>,
    pub(in crate::v2) summary_json: Option<String>,
    pub(in crate::v2) error: Option<String>,
    pub(in crate::v2) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_sessions)]
pub(in crate::v2) struct NewTerminalSessionRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) route_json: String,
    pub(in crate::v2) title: Option<String>,
    pub(in crate::v2) launch_json: Option<String>,
    pub(in crate::v2) source: String,
    pub(in crate::v2) durability_profile: String,
    pub(in crate::v2) retention_policy_id: String,
    pub(in crate::v2) private_mode: i32,
    pub(in crate::v2) created_at_ms: i64,
    pub(in crate::v2) updated_at_ms: i64,
    pub(in crate::v2) closed_at_ms: Option<i64>,
    pub(in crate::v2) state: String,
    pub(in crate::v2) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_panes)]
pub(in crate::v2) struct NewTerminalPaneRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) session_id: String,
    pub(in crate::v2) tab_id: Option<String>,
    pub(in crate::v2) stream_id: String,
    pub(in crate::v2) title: Option<String>,
    pub(in crate::v2) rows: i32,
    pub(in crate::v2) cols: i32,
    pub(in crate::v2) last_event_seq: i64,
    pub(in crate::v2) created_at_ms: i64,
    pub(in crate::v2) closed_at_ms: Option<i64>,
    pub(in crate::v2) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_backend_capability_reports)]
pub(in crate::v2) struct NewBackendCapabilityReportRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) session_id: Option<String>,
    pub(in crate::v2) backend_kind: String,
    pub(in crate::v2) backend_version: Option<String>,
    pub(in crate::v2) backend_binary_path_hash: Option<String>,
    pub(in crate::v2) route_kind: String,
    pub(in crate::v2) probe_status: String,
    pub(in crate::v2) capture_strategy: String,
    pub(in crate::v2) capture_semantics: String,
    pub(in crate::v2) can_preserve_process_when_live: i32,
    pub(in crate::v2) can_capture_scrollback: i32,
    pub(in crate::v2) command_boundary_confidence: String,
    pub(in crate::v2) evidence_json: Option<String>,
    pub(in crate::v2) created_at_ms: i64,
    pub(in crate::v2) expires_at_ms: i64,
    pub(in crate::v2) stale_reason: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_backend_capability_reports)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
pub(in crate::v2) struct BackendCapabilityReportRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) session_id: Option<String>,
    pub(in crate::v2) backend_kind: String,
    pub(in crate::v2) backend_version: Option<String>,
    pub(in crate::v2) backend_binary_path_hash: Option<String>,
    pub(in crate::v2) route_kind: String,
    pub(in crate::v2) probe_status: String,
    pub(in crate::v2) capture_strategy: String,
    pub(in crate::v2) capture_semantics: String,
    pub(in crate::v2) can_preserve_process_when_live: i32,
    pub(in crate::v2) can_capture_scrollback: i32,
    pub(in crate::v2) command_boundary_confidence: String,
    pub(in crate::v2) evidence_json: Option<String>,
    pub(in crate::v2) created_at_ms: i64,
    pub(in crate::v2) expires_at_ms: i64,
    pub(in crate::v2) stale_reason: Option<String>,
}
