use super::super::super::*;

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_export_requests)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(in crate::v2) struct ExportRequestRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) session_id: Option<String>,
    pub(in crate::v2) export_kind: String,
    pub(in crate::v2) state: String,
    pub(in crate::v2) redaction_profile_id: Option<String>,
    pub(in crate::v2) include_raw: i32,
    pub(in crate::v2) approved_at_ms: Option<i64>,
    pub(in crate::v2) requested_at_ms: i64,
    pub(in crate::v2) completed_at_ms: Option<i64>,
    pub(in crate::v2) manifest_json: Option<String>,
    pub(in crate::v2) output_ref_hash: Option<String>,
    pub(in crate::v2) error: Option<String>,
    pub(in crate::v2) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_export_requests)]
pub(in crate::v2) struct NewExportRequestRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) session_id: Option<String>,
    pub(in crate::v2) export_kind: String,
    pub(in crate::v2) state: String,
    pub(in crate::v2) redaction_profile_id: Option<String>,
    pub(in crate::v2) include_raw: i32,
    pub(in crate::v2) approved_at_ms: Option<i64>,
    pub(in crate::v2) requested_at_ms: i64,
    pub(in crate::v2) completed_at_ms: Option<i64>,
    pub(in crate::v2) manifest_json: Option<String>,
    pub(in crate::v2) output_ref_hash: Option<String>,
    pub(in crate::v2) error: Option<String>,
    pub(in crate::v2) metadata_json: Option<String>,
}
