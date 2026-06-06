use super::super::super::*;

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
