use super::super::super::*;
use super::json::parse_optional_json;

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
        approval_record_from_parts(
            row.id,
            row.package_id,
            row.action_kind,
            row.state,
            row.requester_ref_hash,
            row.approver_ref_hash,
            row.requested_at_ms,
            row.decided_at_ms,
            row.expires_at_ms,
            row.metadata_json,
        )
    }
}

impl TryFrom<NewAiActionApprovalRow> for AiActionApprovalRecord {
    type Error = TerminalPersistenceV2Error;

    fn try_from(row: NewAiActionApprovalRow) -> Result<Self, Self::Error> {
        approval_record_from_parts(
            row.id,
            row.package_id,
            row.action_kind,
            row.state,
            row.requester_ref_hash,
            row.approver_ref_hash,
            row.requested_at_ms,
            row.decided_at_ms,
            row.expires_at_ms,
            row.metadata_json,
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn approval_record_from_parts(
    id: String,
    package_id: Option<String>,
    action_kind: String,
    state: String,
    requester_ref_hash: Option<String>,
    approver_ref_hash: Option<String>,
    requested_at_ms: i64,
    decided_at_ms: Option<i64>,
    expires_at_ms: Option<i64>,
    metadata_json: Option<String>,
) -> Result<AiActionApprovalRecord, TerminalPersistenceV2Error> {
    Ok(AiActionApprovalRecord {
        id,
        package_id,
        action_kind,
        state,
        requester_ref_hash,
        approver_ref_hash,
        requested_at_ms,
        decided_at_ms,
        expires_at_ms,
        metadata_json: parse_optional_json(metadata_json)?,
    })
}
