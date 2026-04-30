use super::super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntegrityCheckRecord {
    pub id: String,
    pub check_kind: String,
    pub scope_kind: String,
    pub scope_ref: Option<String>,
    pub result: String,
    pub checked_at_ms: i64,
    pub details_json: Option<Value>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataHealthRecord {
    pub id: String,
    pub session_id: Option<String>,
    pub pane_id: Option<String>,
    pub detection_kind: String,
    pub severity: String,
    pub first_bad_event_seq: Option<i64>,
    pub affected_ref: Option<String>,
    pub action_state: String,
    pub detected_at_ms: i64,
    pub resolved_at_ms: Option<i64>,
    pub details_json: Option<Value>,
}

impl TryFrom<DataHealthRecordRow> for DataHealthRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: DataHealthRecordRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            session_id: row.session_id,
            pane_id: row.pane_id,
            detection_kind: row.detection_kind,
            severity: row.severity,
            first_bad_event_seq: row.first_bad_event_seq,
            affected_ref: row.affected_ref,
            action_state: row.action_state,
            detected_at_ms: row.detected_at_ms,
            resolved_at_ms: row.resolved_at_ms,
            details_json: row.details_json.map(|value| serde_json::from_str(&value)).transpose()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupRecord {
    pub id: String,
    pub backup_kind: String,
    pub state: String,
    pub target_ref_hash: Option<String>,
    pub manifest_json: Option<Value>,
    pub checksum_algorithm: Option<String>,
    pub checksum: Option<String>,
    pub source_db_path_hash: Option<String>,
    pub started_at_ms: i64,
    pub finished_at_ms: Option<i64>,
    pub quick_check_result: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintenanceRunInput {
    pub id: Option<String>,
    pub run_kind: Option<String>,
    pub selected_policy_id: Option<String>,
    pub run_wal_checkpoint: bool,
    pub run_optimize: bool,
    pub metadata: Option<Value>,
}

impl Default for MaintenanceRunInput {
    fn default() -> Self {
        Self {
            id: None,
            run_kind: Some("scheduled_maintenance".to_string()),
            selected_policy_id: None,
            run_wal_checkpoint: true,
            run_optimize: true,
            metadata: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaintenanceRunRecord {
    pub id: String,
    pub run_kind: String,
    pub state: String,
    pub selected_policy_id: Option<String>,
    pub started_at_ms: i64,
    pub finished_at_ms: Option<i64>,
    pub summary_json: Option<Value>,
    pub error: Option<String>,
    pub metadata_json: Option<Value>,
}

impl TryFrom<MaintenanceRunRow> for MaintenanceRunRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: MaintenanceRunRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            run_kind: row.run_kind,
            state: row.state,
            selected_policy_id: row.selected_policy_id,
            started_at_ms: row.started_at_ms,
            finished_at_ms: row.finished_at_ms,
            summary_json: row.summary_json.as_deref().map(serde_json::from_str).transpose()?,
            error: row.error,
            metadata_json: row.metadata_json.as_deref().map(serde_json::from_str).transpose()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoragePressureRecord {
    pub id: String,
    pub state: String,
    pub db_file_bytes: Option<i64>,
    pub wal_file_bytes: Option<i64>,
    pub disk_free_bytes: Option<i64>,
    pub temp_free_bytes: Option<i64>,
    pub quota_bytes: Option<i64>,
    pub action_taken: String,
    pub reason: Option<String>,
    pub created_at_ms: i64,
    pub metadata_json: Option<Value>,
}

impl TryFrom<StoragePressureEventRow> for StoragePressureRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: StoragePressureEventRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            state: row.state,
            db_file_bytes: row.db_file_bytes,
            wal_file_bytes: row.wal_file_bytes,
            disk_free_bytes: row.disk_free_bytes,
            temp_free_bytes: row.temp_free_bytes,
            quota_bytes: row.quota_bytes,
            action_taken: row.action_taken,
            reason: row.reason,
            created_at_ms: row.created_at_ms,
            metadata_json: row
                .metadata_json
                .map(|value| serde_json::from_str(&value))
                .transpose()?,
        })
    }
}

impl From<NewStoragePressureEventRow> for StoragePressureRecord {
    fn from(row: NewStoragePressureEventRow) -> Self {
        Self {
            id: row.id,
            state: row.state,
            db_file_bytes: row.db_file_bytes,
            wal_file_bytes: row.wal_file_bytes,
            disk_free_bytes: row.disk_free_bytes,
            temp_free_bytes: row.temp_free_bytes,
            quota_bytes: row.quota_bytes,
            action_taken: row.action_taken,
            reason: row.reason,
            created_at_ms: row.created_at_ms,
            metadata_json: row.metadata_json.and_then(|value| serde_json::from_str(&value).ok()),
        }
    }
}
