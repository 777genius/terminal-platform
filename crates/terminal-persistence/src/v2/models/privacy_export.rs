use super::super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeleteRequestRecord {
    pub id: String,
    pub session_id: Option<String>,
    pub request_kind: String,
    pub state: String,
    pub policy_id: Option<String>,
    pub requested_at_ms: i64,
    pub approved_at_ms: Option<i64>,
    pub completed_at_ms: Option<i64>,
    pub requester_ref_hash: Option<String>,
    pub reason: Option<String>,
    pub metadata_json: Option<Value>,
}

impl TryFrom<DeleteRequestRow> for DeleteRequestRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: DeleteRequestRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            session_id: row.session_id,
            request_kind: row.request_kind,
            state: row.state,
            policy_id: row.policy_id,
            requested_at_ms: row.requested_at_ms,
            approved_at_ms: row.approved_at_ms,
            completed_at_ms: row.completed_at_ms,
            requester_ref_hash: row.requester_ref_hash,
            reason: row.reason,
            metadata_json: row
                .metadata_json
                .map(|value| serde_json::from_str(&value))
                .transpose()?,
        })
    }
}

impl TryFrom<NewDeleteRequestRow> for DeleteRequestRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: NewDeleteRequestRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            session_id: row.session_id,
            request_kind: row.request_kind,
            state: row.state,
            policy_id: row.policy_id,
            requested_at_ms: row.requested_at_ms,
            approved_at_ms: row.approved_at_ms,
            completed_at_ms: row.completed_at_ms,
            requester_ref_hash: row.requester_ref_hash,
            reason: row.reason,
            metadata_json: row
                .metadata_json
                .map(|value| serde_json::from_str(&value))
                .transpose()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeletionTombstoneRecord {
    pub id: String,
    pub delete_request_id: Option<String>,
    pub session_id: Option<String>,
    pub deleted_scope: String,
    pub policy_id: Option<String>,
    pub deleted_at_ms: i64,
    pub evidence_json: Option<Value>,
    pub metadata_json: Option<Value>,
}

impl TryFrom<NewDeletionTombstoneRow> for DeletionTombstoneRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: NewDeletionTombstoneRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            delete_request_id: row.delete_request_id,
            session_id: row.session_id,
            deleted_scope: row.deleted_scope,
            policy_id: row.policy_id,
            deleted_at_ms: row.deleted_at_ms,
            evidence_json: row
                .evidence_json
                .map(|value| serde_json::from_str(&value))
                .transpose()?,
            metadata_json: row
                .metadata_json
                .map(|value| serde_json::from_str(&value))
                .transpose()?,
        })
    }
}

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CryptoKeyRecord {
    pub id: String,
    pub key_kind: String,
    pub key_ref_hash: String,
    pub protection_kind: String,
    pub state: String,
    pub created_at_ms: i64,
    pub rotated_at_ms: Option<i64>,
    pub destroyed_at_ms: Option<i64>,
    pub capability_report_json: Option<Value>,
    pub error_json: Option<Value>,
    pub metadata_json: Option<Value>,
}

impl TryFrom<CryptoKeyRow> for CryptoKeyRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: CryptoKeyRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            key_kind: row.key_kind,
            key_ref_hash: blake3_hash_text(&row.key_ref),
            protection_kind: row.protection_kind,
            state: row.state,
            created_at_ms: row.created_at_ms,
            rotated_at_ms: row.rotated_at_ms,
            destroyed_at_ms: row.destroyed_at_ms,
            capability_report_json: row
                .capability_report_json
                .as_deref()
                .map(serde_json::from_str)
                .transpose()?,
            error_json: row.error_json.as_deref().map(serde_json::from_str).transpose()?,
            metadata_json: row.metadata_json.as_deref().map(serde_json::from_str).transpose()?,
        })
    }
}

impl TryFrom<NewCryptoKeyRow> for CryptoKeyRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: NewCryptoKeyRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            key_kind: row.key_kind,
            key_ref_hash: blake3_hash_text(&row.key_ref),
            protection_kind: row.protection_kind,
            state: row.state,
            created_at_ms: row.created_at_ms,
            rotated_at_ms: row.rotated_at_ms,
            destroyed_at_ms: row.destroyed_at_ms,
            capability_report_json: row
                .capability_report_json
                .as_deref()
                .map(serde_json::from_str)
                .transpose()?,
            error_json: row.error_json.as_deref().map(serde_json::from_str).transpose()?,
            metadata_json: row.metadata_json.as_deref().map(serde_json::from_str).transpose()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CryptoKeyEventRecord {
    pub id: String,
    pub key_id: Option<String>,
    pub event_kind: String,
    pub actor: String,
    pub occurred_at_ms: i64,
    pub status: String,
    pub error_json: Option<Value>,
    pub metadata_json: Option<Value>,
}

impl TryFrom<NewCryptoKeyEventRow> for CryptoKeyEventRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: NewCryptoKeyEventRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            key_id: row.key_id,
            event_kind: row.event_kind,
            actor: row.actor,
            occurred_at_ms: row.occurred_at_ms,
            status: row.status,
            error_json: row.error_json.as_deref().map(serde_json::from_str).transpose()?,
            metadata_json: row.metadata_json.as_deref().map(serde_json::from_str).transpose()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CryptoEraseRecord {
    pub key_id: String,
    pub key_ref_hash: String,
    pub delete_request_id: String,
    pub tombstone_id: String,
    pub state: String,
    pub secure_deletion_limitation: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptionCapabilityRecord {
    pub feature_gate_state: String,
    pub active_database_key_count: i64,
    pub active_non_test_database_key_count: i64,
    pub test_plaintext_database_key_count: i64,
    pub unavailable_key_count: i64,
    pub can_enable_encrypted_history: bool,
    pub plaintext_fallback_allowed: bool,
    pub key_material_exported: bool,
    pub action_required: String,
}

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
