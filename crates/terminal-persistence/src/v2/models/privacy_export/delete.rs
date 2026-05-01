use super::super::super::*;

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
