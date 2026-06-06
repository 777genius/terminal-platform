use super::super::super::*;

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_restore_drills)]
pub(in crate::v2) struct NewRestoreDrillRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) session_id: String,
    pub(in crate::v2) drill_kind: String,
    pub(in crate::v2) result: String,
    pub(in crate::v2) restore_guarantee_level: String,
    pub(in crate::v2) checked_at_ms: i64,
    pub(in crate::v2) duration_ms: Option<i64>,
    pub(in crate::v2) source_snapshot_id: Option<String>,
    pub(in crate::v2) evidence_json: Option<String>,
    pub(in crate::v2) error: Option<String>,
    pub(in crate::v2) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_integrity_checks)]
pub(in crate::v2) struct NewIntegrityCheckRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) check_kind: String,
    pub(in crate::v2) scope_kind: String,
    pub(in crate::v2) scope_ref: Option<String>,
    pub(in crate::v2) result: String,
    pub(in crate::v2) checked_at_ms: i64,
    pub(in crate::v2) details_json: Option<String>,
    pub(in crate::v2) error: Option<String>,
    pub(in crate::v2) metadata_json: Option<String>,
}
