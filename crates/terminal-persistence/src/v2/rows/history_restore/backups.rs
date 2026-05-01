use super::super::super::*;

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_backup_records)]
pub(in crate::v2) struct NewBackupRecordRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) backup_kind: String,
    pub(in crate::v2) state: String,
    pub(in crate::v2) target_ref_hash: Option<String>,
    pub(in crate::v2) manifest_json: Option<String>,
    pub(in crate::v2) checksum_algorithm: Option<String>,
    pub(in crate::v2) checksum: Option<String>,
    pub(in crate::v2) source_db_path_hash: Option<String>,
    pub(in crate::v2) started_at_ms: i64,
    pub(in crate::v2) finished_at_ms: Option<i64>,
    pub(in crate::v2) quick_check_result: Option<String>,
    pub(in crate::v2) error: Option<String>,
    pub(in crate::v2) metadata_json: Option<String>,
}
