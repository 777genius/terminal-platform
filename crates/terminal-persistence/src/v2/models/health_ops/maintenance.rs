use super::super::super::*;

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
