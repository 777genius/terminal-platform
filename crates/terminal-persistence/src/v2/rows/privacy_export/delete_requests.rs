use super::super::super::*;

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_delete_requests)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
pub(in crate::v2) struct DeleteRequestRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) session_id: Option<String>,
    pub(in crate::v2) request_kind: String,
    pub(in crate::v2) state: String,
    pub(in crate::v2) policy_id: Option<String>,
    pub(in crate::v2) requested_at_ms: i64,
    pub(in crate::v2) approved_at_ms: Option<i64>,
    pub(in crate::v2) completed_at_ms: Option<i64>,
    pub(in crate::v2) requester_ref_hash: Option<String>,
    pub(in crate::v2) reason: Option<String>,
    pub(in crate::v2) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_delete_requests)]
pub(in crate::v2) struct NewDeleteRequestRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) session_id: Option<String>,
    pub(in crate::v2) request_kind: String,
    pub(in crate::v2) state: String,
    pub(in crate::v2) policy_id: Option<String>,
    pub(in crate::v2) requested_at_ms: i64,
    pub(in crate::v2) approved_at_ms: Option<i64>,
    pub(in crate::v2) completed_at_ms: Option<i64>,
    pub(in crate::v2) requester_ref_hash: Option<String>,
    pub(in crate::v2) reason: Option<String>,
    pub(in crate::v2) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_deletion_tombstones)]
pub(in crate::v2) struct NewDeletionTombstoneRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) delete_request_id: Option<String>,
    pub(in crate::v2) session_id: Option<String>,
    pub(in crate::v2) deleted_scope: String,
    pub(in crate::v2) policy_id: Option<String>,
    pub(in crate::v2) deleted_at_ms: i64,
    pub(in crate::v2) evidence_json: Option<String>,
    pub(in crate::v2) metadata_json: Option<String>,
}
