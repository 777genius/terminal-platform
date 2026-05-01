use super::super::super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupRecord {
    pub id: String,
    pub backup_kind: String,
    pub state: String,
    pub target_ref_hash: Option<String>,
    pub manifest_json: Option<Value>,
    pub checksum_algorithm: Option<String>,
    pub checksum: Option<String>,
    pub source_db_path_hash: Option<String>,
    pub started_at_ms: i64,
    pub finished_at_ms: Option<i64>,
    pub quick_check_result: Option<String>,
    pub error: Option<String>,
}
