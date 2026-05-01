use super::super::super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportRequestRecord {
    pub id: String,
    pub session_id: Option<String>,
    pub export_kind: String,
    pub state: String,
    pub redaction_profile_id: Option<String>,
    pub include_raw: bool,
    pub approved_at_ms: Option<i64>,
    pub requested_at_ms: i64,
    pub completed_at_ms: Option<i64>,
    pub manifest_json: Option<Value>,
    pub output_ref_hash: Option<String>,
    pub error: Option<String>,
    pub metadata_json: Option<Value>,
}

impl TryFrom<NewExportRequestRow> for ExportRequestRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: NewExportRequestRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            session_id: row.session_id,
            export_kind: row.export_kind,
            state: row.state,
            redaction_profile_id: row.redaction_profile_id,
            include_raw: row.include_raw != 0,
            approved_at_ms: row.approved_at_ms,
            requested_at_ms: row.requested_at_ms,
            completed_at_ms: row.completed_at_ms,
            manifest_json: row
                .manifest_json
                .map(|value| serde_json::from_str(&value))
                .transpose()?,
            output_ref_hash: row.output_ref_hash,
            error: row.error,
            metadata_json: row
                .metadata_json
                .map(|value| serde_json::from_str(&value))
                .transpose()?,
        })
    }
}

impl TryFrom<ExportRequestRow> for ExportRequestRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: ExportRequestRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            session_id: row.session_id,
            export_kind: row.export_kind,
            state: row.state,
            redaction_profile_id: row.redaction_profile_id,
            include_raw: row.include_raw != 0,
            approved_at_ms: row.approved_at_ms,
            requested_at_ms: row.requested_at_ms,
            completed_at_ms: row.completed_at_ms,
            manifest_json: row
                .manifest_json
                .map(|value| serde_json::from_str(&value))
                .transpose()?,
            output_ref_hash: row.output_ref_hash,
            error: row.error,
            metadata_json: row
                .metadata_json
                .map(|value| serde_json::from_str(&value))
                .transpose()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportArtifactVerificationRecord {
    pub export_request_id: String,
    pub artifact_id: String,
    pub artifact_ref_hash: String,
    pub export_state: String,
    pub artifact_state: String,
    pub encryption_state: String,
    pub raw_export: bool,
    pub encrypted_required: bool,
    pub verified_at_ms: i64,
    pub checksum_algorithm: Option<String>,
    pub checksum: Option<String>,
    pub manifest_json: Value,
}
