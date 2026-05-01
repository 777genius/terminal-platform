use super::super::super::*;

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_writer_generations)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
pub(in crate::v2) struct WriterGenerationRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) process_id: String,
    pub(in crate::v2) lease_token: String,
    pub(in crate::v2) state: String,
    pub(in crate::v2) acquired_at_ms: i64,
    pub(in crate::v2) heartbeat_at_ms: i64,
    pub(in crate::v2) lease_expires_at_ms: i64,
    pub(in crate::v2) released_at_ms: Option<i64>,
    pub(in crate::v2) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_writer_generations)]
pub(in crate::v2) struct NewWriterGenerationRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) process_id: String,
    pub(in crate::v2) lease_token: String,
    pub(in crate::v2) state: String,
    pub(in crate::v2) acquired_at_ms: i64,
    pub(in crate::v2) heartbeat_at_ms: i64,
    pub(in crate::v2) lease_expires_at_ms: i64,
    pub(in crate::v2) released_at_ms: Option<i64>,
    pub(in crate::v2) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_clock_anchors)]
pub(in crate::v2) struct NewClockAnchorRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) writer_generation: String,
    pub(in crate::v2) wall_time_ms: i64,
    pub(in crate::v2) monotonic_ms: i64,
    pub(in crate::v2) source: String,
    pub(in crate::v2) created_at_ms: i64,
}
