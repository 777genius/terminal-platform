use super::super::super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoragePressureEventInput {
    pub id: Option<String>,
    pub state: Option<String>,
    pub db_file_bytes: Option<i64>,
    pub wal_file_bytes: Option<i64>,
    pub disk_free_bytes: Option<i64>,
    pub temp_free_bytes: Option<i64>,
    pub quota_bytes: Option<i64>,
    pub action_taken: Option<String>,
    pub reason: Option<String>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteRequestInput {
    pub id: Option<String>,
    pub session_id: Option<String>,
    pub request_kind: Option<String>,
    pub policy_id: Option<String>,
    pub requester_ref: Option<String>,
    pub reason: Option<String>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportRequestInput {
    pub id: Option<String>,
    pub session_id: Option<String>,
    pub export_kind: Option<String>,
    pub redaction_profile_id: Option<String>,
    pub include_raw: bool,
    pub output_ref: Option<String>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportApprovalInput {
    pub export_request_id: String,
    pub approver_ref: Option<String>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportArtifactVerificationInput {
    pub export_request_id: String,
    pub artifact_ref: String,
    pub require_encrypted: bool,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportBundleInput {
    pub id: Option<String>,
    pub scope: Value,
    pub redaction_profile_id: Option<String>,
    pub include_raw: bool,
    pub output_ref: Option<String>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportBundleCompletionInput {
    pub support_bundle_id: String,
    pub artifact_ref: Option<String>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoKeyInput {
    pub id: Option<String>,
    pub key_kind: String,
    pub key_ref: String,
    pub protection_kind: String,
    pub state: Option<String>,
    pub capability_report: Option<Value>,
    pub error: Option<Value>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoKeyEventInput {
    pub id: Option<String>,
    pub key_id: Option<String>,
    pub event_kind: String,
    pub actor: String,
    pub status: String,
    pub error: Option<Value>,
    pub metadata: Option<Value>,
    pub occurred_at_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoEraseInput {
    pub id: Option<String>,
    pub key_id: String,
    pub session_id: Option<String>,
    pub requester_ref: Option<String>,
    pub reason: Option<String>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalArtifactInput {
    pub id: Option<String>,
    pub artifact_kind: String,
    pub artifact_ref: String,
    pub state: Option<String>,
    pub encryption_state: Option<String>,
    pub key_ref: Option<String>,
    pub checksum_algorithm: Option<String>,
    pub checksum: Option<String>,
    pub size_bytes: Option<i64>,
    pub verified_at_ms: Option<i64>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchDocumentInput {
    pub document_id: Option<String>,
    pub session_id: String,
    pub pane_id: Option<String>,
    pub command_block_id: Option<String>,
    pub document_kind: Option<String>,
    pub event_seq_low: Option<i64>,
    pub event_seq_high: Option<i64>,
    pub byte_low: Option<i64>,
    pub byte_high: Option<i64>,
    pub redaction_profile_id: Option<String>,
    pub raw_text: String,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiContextPackageInput {
    pub id: Option<String>,
    pub session_id: Option<String>,
    pub pane_id: Option<String>,
    pub redaction_profile_id: Option<String>,
    pub include_raw: bool,
    pub max_items: Option<i64>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiActionApprovalInput {
    pub id: Option<String>,
    pub package_id: String,
    pub action_kind: String,
    pub requester_ref: Option<String>,
    pub expires_at_ms: Option<i64>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiActionDecisionInput {
    pub approval_id: String,
    pub approved: bool,
    pub approver_ref: Option<String>,
    pub metadata: Option<Value>,
}
