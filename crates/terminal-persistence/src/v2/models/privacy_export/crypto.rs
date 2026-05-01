use super::super::super::*;

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
