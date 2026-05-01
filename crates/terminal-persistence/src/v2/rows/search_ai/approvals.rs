use super::super::super::*;

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_ai_action_approvals)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(in crate::v2) struct AiActionApprovalRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) package_id: Option<String>,
    pub(in crate::v2) action_kind: String,
    pub(in crate::v2) state: String,
    pub(in crate::v2) requester_ref_hash: Option<String>,
    pub(in crate::v2) approver_ref_hash: Option<String>,
    pub(in crate::v2) requested_at_ms: i64,
    pub(in crate::v2) decided_at_ms: Option<i64>,
    pub(in crate::v2) expires_at_ms: Option<i64>,
    pub(in crate::v2) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_ai_action_approvals)]
pub(in crate::v2) struct NewAiActionApprovalRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) package_id: Option<String>,
    pub(in crate::v2) action_kind: String,
    pub(in crate::v2) state: String,
    pub(in crate::v2) requester_ref_hash: Option<String>,
    pub(in crate::v2) approver_ref_hash: Option<String>,
    pub(in crate::v2) requested_at_ms: i64,
    pub(in crate::v2) decided_at_ms: Option<i64>,
    pub(in crate::v2) expires_at_ms: Option<i64>,
    pub(in crate::v2) metadata_json: Option<String>,
}
