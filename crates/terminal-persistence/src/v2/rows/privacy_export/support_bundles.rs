use super::super::super::*;

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_support_bundles)]
pub(in crate::v2) struct NewSupportBundleRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) scope_json: String,
    pub(in crate::v2) state: String,
    pub(in crate::v2) redaction_profile_id: Option<String>,
    pub(in crate::v2) include_raw: i32,
    pub(in crate::v2) requested_at_ms: i64,
    pub(in crate::v2) completed_at_ms: Option<i64>,
    pub(in crate::v2) manifest_json: Option<String>,
    pub(in crate::v2) output_ref_hash: Option<String>,
    pub(in crate::v2) error: Option<String>,
    pub(in crate::v2) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_support_bundles)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(in crate::v2) struct SupportBundleRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) scope_json: String,
    pub(in crate::v2) state: String,
    pub(in crate::v2) redaction_profile_id: Option<String>,
    pub(in crate::v2) include_raw: i32,
    pub(in crate::v2) requested_at_ms: i64,
    pub(in crate::v2) completed_at_ms: Option<i64>,
    pub(in crate::v2) manifest_json: Option<String>,
    pub(in crate::v2) output_ref_hash: Option<String>,
    pub(in crate::v2) error: Option<String>,
    pub(in crate::v2) metadata_json: Option<String>,
}
