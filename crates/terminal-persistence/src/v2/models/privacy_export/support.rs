use super::super::super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupportBundleRecord {
    pub id: String,
    pub scope_json: Value,
    pub state: String,
    pub redaction_profile_id: Option<String>,
    pub include_raw: bool,
    pub requested_at_ms: i64,
    pub completed_at_ms: Option<i64>,
    pub manifest_json: Option<Value>,
    pub output_ref_hash: Option<String>,
    pub error: Option<String>,
    pub metadata_json: Option<Value>,
}

impl TryFrom<NewSupportBundleRow> for SupportBundleRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: NewSupportBundleRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            scope_json: serde_json::from_str(&row.scope_json)?,
            state: row.state,
            redaction_profile_id: row.redaction_profile_id,
            include_raw: row.include_raw != 0,
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

impl TryFrom<SupportBundleRow> for SupportBundleRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: SupportBundleRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            scope_json: serde_json::from_str(&row.scope_json)?,
            state: row.state,
            redaction_profile_id: row.redaction_profile_id,
            include_raw: row.include_raw != 0,
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
pub struct SupportBundleDiagnosticsRecord {
    pub support_bundle_id: String,
    pub generated_at_ms: i64,
    pub include_raw: bool,
    pub raw_content_included: bool,
    pub manifest_json: Value,
}
