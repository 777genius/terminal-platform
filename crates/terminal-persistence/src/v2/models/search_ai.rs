use super::super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchDocumentRecord {
    pub document_id: String,
    pub session_id: String,
    pub pane_id: Option<String>,
    pub command_block_id: Option<String>,
    pub document_kind: String,
    pub event_seq_low: Option<i64>,
    pub event_seq_high: Option<i64>,
    pub byte_low: Option<i64>,
    pub byte_high: Option<i64>,
    pub redaction_profile_id: Option<String>,
    pub redaction_state: String,
    pub source_hash_algorithm: String,
    pub source_hash: String,
    pub text_preview: String,
    pub updated_at_ms: i64,
    pub metadata_json: Option<Value>,
}

impl TryFrom<SearchDocumentRow> for SearchDocumentRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: SearchDocumentRow) -> Result<Self, Self::Error> {
        Ok(Self {
            document_id: row.document_id,
            session_id: row.session_id,
            pane_id: row.pane_id,
            command_block_id: row.command_block_id,
            document_kind: row.document_kind,
            event_seq_low: row.event_seq_low,
            event_seq_high: row.event_seq_high,
            byte_low: row.byte_low,
            byte_high: row.byte_high,
            redaction_profile_id: row.redaction_profile_id,
            redaction_state: row.redaction_state,
            source_hash_algorithm: row.source_hash_algorithm,
            source_hash: row.source_hash,
            text_preview: row.text_preview,
            updated_at_ms: row.updated_at_ms,
            metadata_json: row
                .metadata_json
                .map(|value| serde_json::from_str(&value))
                .transpose()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiContextPackageRecord {
    pub id: String,
    pub session_id: Option<String>,
    pub pane_id: Option<String>,
    pub state: String,
    pub redaction_profile_id: Option<String>,
    pub include_raw: bool,
    pub requested_at_ms: i64,
    pub built_at_ms: Option<i64>,
    pub item_count: i64,
    pub manifest_json: Option<Value>,
    pub metadata_json: Option<Value>,
}

impl TryFrom<AiContextPackageRow> for AiContextPackageRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: AiContextPackageRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            session_id: row.session_id,
            pane_id: row.pane_id,
            state: row.state,
            redaction_profile_id: row.redaction_profile_id,
            include_raw: row.include_raw != 0,
            requested_at_ms: row.requested_at_ms,
            built_at_ms: row.built_at_ms,
            item_count: row.item_count,
            manifest_json: row
                .manifest_json
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
pub struct AiContextItemRecord {
    pub id: String,
    pub package_id: String,
    pub source_kind: String,
    pub source_ref: Option<String>,
    pub session_id: Option<String>,
    pub pane_id: Option<String>,
    pub command_block_id: Option<String>,
    pub event_seq_low: Option<i64>,
    pub event_seq_high: Option<i64>,
    pub byte_low: Option<i64>,
    pub byte_high: Option<i64>,
    pub redaction_state: String,
    pub data_only: bool,
    pub content_preview: String,
    pub metadata_json: Option<Value>,
}

impl TryFrom<AiContextItemRow> for AiContextItemRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: AiContextItemRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            package_id: row.package_id,
            source_kind: row.source_kind,
            source_ref: row.source_ref,
            session_id: row.session_id,
            pane_id: row.pane_id,
            command_block_id: row.command_block_id,
            event_seq_low: row.event_seq_low,
            event_seq_high: row.event_seq_high,
            byte_low: row.byte_low,
            byte_high: row.byte_high,
            redaction_state: row.redaction_state,
            data_only: row.data_only != 0,
            content_preview: row.content_preview,
            metadata_json: row
                .metadata_json
                .map(|value| serde_json::from_str(&value))
                .transpose()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptInjectionFindingRecord {
    pub id: String,
    pub package_id: Option<String>,
    pub item_id: Option<String>,
    pub severity: String,
    pub pattern_kind: String,
    pub action_state: String,
    pub detected_at_ms: i64,
    pub evidence_preview: String,
    pub metadata_json: Option<Value>,
}

impl TryFrom<PromptInjectionFindingRow> for PromptInjectionFindingRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: PromptInjectionFindingRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            package_id: row.package_id,
            item_id: row.item_id,
            severity: row.severity,
            pattern_kind: row.pattern_kind,
            action_state: row.action_state,
            detected_at_ms: row.detected_at_ms,
            evidence_preview: row.evidence_preview,
            metadata_json: row
                .metadata_json
                .map(|value| serde_json::from_str(&value))
                .transpose()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiActionApprovalRecord {
    pub id: String,
    pub package_id: Option<String>,
    pub action_kind: String,
    pub state: String,
    pub requester_ref_hash: Option<String>,
    pub approver_ref_hash: Option<String>,
    pub requested_at_ms: i64,
    pub decided_at_ms: Option<i64>,
    pub expires_at_ms: Option<i64>,
    pub metadata_json: Option<Value>,
}

impl TryFrom<AiActionApprovalRow> for AiActionApprovalRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: AiActionApprovalRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            package_id: row.package_id,
            action_kind: row.action_kind,
            state: row.state,
            requester_ref_hash: row.requester_ref_hash,
            approver_ref_hash: row.approver_ref_hash,
            requested_at_ms: row.requested_at_ms,
            decided_at_ms: row.decided_at_ms,
            expires_at_ms: row.expires_at_ms,
            metadata_json: row
                .metadata_json
                .map(|value| serde_json::from_str(&value))
                .transpose()?,
        })
    }
}

impl TryFrom<NewAiActionApprovalRow> for AiActionApprovalRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: NewAiActionApprovalRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            package_id: row.package_id,
            action_kind: row.action_kind,
            state: row.state,
            requester_ref_hash: row.requester_ref_hash,
            approver_ref_hash: row.approver_ref_hash,
            requested_at_ms: row.requested_at_ms,
            decided_at_ms: row.decided_at_ms,
            expires_at_ms: row.expires_at_ms,
            metadata_json: row
                .metadata_json
                .map(|value| serde_json::from_str(&value))
                .transpose()?,
        })
    }
}
