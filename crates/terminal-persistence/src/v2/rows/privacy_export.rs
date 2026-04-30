use super::super::*;

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

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_crypto_keys)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
pub(in crate::v2) struct CryptoKeyRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) key_kind: String,
    pub(in crate::v2) key_ref: String,
    pub(in crate::v2) protection_kind: String,
    pub(in crate::v2) state: String,
    pub(in crate::v2) created_at_ms: i64,
    pub(in crate::v2) rotated_at_ms: Option<i64>,
    pub(in crate::v2) destroyed_at_ms: Option<i64>,
    pub(in crate::v2) capability_report_json: Option<String>,
    pub(in crate::v2) error_json: Option<String>,
    pub(in crate::v2) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_crypto_keys)]
pub(in crate::v2) struct NewCryptoKeyRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) key_kind: String,
    pub(in crate::v2) key_ref: String,
    pub(in crate::v2) protection_kind: String,
    pub(in crate::v2) state: String,
    pub(in crate::v2) created_at_ms: i64,
    pub(in crate::v2) rotated_at_ms: Option<i64>,
    pub(in crate::v2) destroyed_at_ms: Option<i64>,
    pub(in crate::v2) capability_report_json: Option<String>,
    pub(in crate::v2) error_json: Option<String>,
    pub(in crate::v2) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_crypto_key_events)]
pub(in crate::v2) struct NewCryptoKeyEventRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) key_id: Option<String>,
    pub(in crate::v2) event_kind: String,
    pub(in crate::v2) actor: String,
    pub(in crate::v2) occurred_at_ms: i64,
    pub(in crate::v2) status: String,
    pub(in crate::v2) error_json: Option<String>,
    pub(in crate::v2) metadata_json: Option<String>,
}

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
