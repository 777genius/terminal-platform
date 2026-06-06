use super::super::super::*;

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_external_artifacts)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(in crate::v2) struct ExternalArtifactRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) artifact_kind: String,
    pub(in crate::v2) artifact_ref_hash: String,
    pub(in crate::v2) state: String,
    pub(in crate::v2) encryption_state: String,
    pub(in crate::v2) key_ref: Option<String>,
    pub(in crate::v2) checksum_algorithm: Option<String>,
    pub(in crate::v2) checksum: Option<String>,
    pub(in crate::v2) size_bytes: Option<i64>,
    pub(in crate::v2) created_at_ms: i64,
    pub(in crate::v2) verified_at_ms: Option<i64>,
    pub(in crate::v2) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_external_artifacts)]
pub(in crate::v2) struct NewExternalArtifactRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) artifact_kind: String,
    pub(in crate::v2) artifact_ref_hash: String,
    pub(in crate::v2) state: String,
    pub(in crate::v2) encryption_state: String,
    pub(in crate::v2) key_ref: Option<String>,
    pub(in crate::v2) checksum_algorithm: Option<String>,
    pub(in crate::v2) checksum: Option<String>,
    pub(in crate::v2) size_bytes: Option<i64>,
    pub(in crate::v2) created_at_ms: i64,
    pub(in crate::v2) verified_at_ms: Option<i64>,
    pub(in crate::v2) metadata_json: Option<String>,
}
