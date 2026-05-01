use super::super::super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalArtifactRecord {
    pub id: String,
    pub artifact_kind: String,
    pub artifact_ref_hash: String,
    pub state: String,
    pub encryption_state: String,
    pub key_ref: Option<String>,
    pub checksum_algorithm: Option<String>,
    pub checksum: Option<String>,
    pub size_bytes: Option<i64>,
    pub created_at_ms: i64,
    pub verified_at_ms: Option<i64>,
    pub metadata_json: Option<Value>,
}

impl TryFrom<NewExternalArtifactRow> for ExternalArtifactRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: NewExternalArtifactRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            artifact_kind: row.artifact_kind,
            artifact_ref_hash: row.artifact_ref_hash,
            state: row.state,
            encryption_state: row.encryption_state,
            key_ref: row.key_ref,
            checksum_algorithm: row.checksum_algorithm,
            checksum: row.checksum,
            size_bytes: row.size_bytes,
            created_at_ms: row.created_at_ms,
            verified_at_ms: row.verified_at_ms,
            metadata_json: row.metadata_json.as_deref().map(serde_json::from_str).transpose()?,
        })
    }
}

impl TryFrom<ExternalArtifactRow> for ExternalArtifactRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: ExternalArtifactRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            artifact_kind: row.artifact_kind,
            artifact_ref_hash: row.artifact_ref_hash,
            state: row.state,
            encryption_state: row.encryption_state,
            key_ref: row.key_ref,
            checksum_algorithm: row.checksum_algorithm,
            checksum: row.checksum,
            size_bytes: row.size_bytes,
            created_at_ms: row.created_at_ms,
            verified_at_ms: row.verified_at_ms,
            metadata_json: row.metadata_json.as_deref().map(serde_json::from_str).transpose()?,
        })
    }
}
