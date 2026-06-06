use super::super::super::*;

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
