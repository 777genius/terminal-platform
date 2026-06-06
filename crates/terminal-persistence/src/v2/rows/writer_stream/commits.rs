use super::super::super::*;

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_commit_log)]
pub(in crate::v2) struct NewCommitLogRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) session_id: String,
    pub(in crate::v2) commit_seq: i64,
    pub(in crate::v2) commit_kind: String,
    pub(in crate::v2) writer_generation: String,
    pub(in crate::v2) occurred_at_ms: i64,
    pub(in crate::v2) created_at_ms: i64,
    pub(in crate::v2) metadata_json: Option<String>,
}

#[derive(Debug, Clone)]
pub(in crate::v2) struct CommitAllocation {
    pub(in crate::v2) id: String,
    pub(in crate::v2) commit_seq: i64,
}
