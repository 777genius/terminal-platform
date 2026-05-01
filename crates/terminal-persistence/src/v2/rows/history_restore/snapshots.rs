use super::super::super::*;

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_screen_snapshots)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
pub(in crate::v2) struct ScreenSnapshotRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) session_id: String,
    pub(in crate::v2) pane_id: String,
    pub(in crate::v2) commit_id: String,
    pub(in crate::v2) projection_source: String,
    pub(in crate::v2) buffer_kind: String,
    pub(in crate::v2) rows: i32,
    pub(in crate::v2) cols: i32,
    pub(in crate::v2) base_event_seq: i64,
    pub(in crate::v2) high_water_event_seq: i64,
    pub(in crate::v2) high_water_byte_seq: Option<i64>,
    pub(in crate::v2) screen_json: String,
    pub(in crate::v2) parser_version: String,
    pub(in crate::v2) projection_version: String,
    pub(in crate::v2) checksum_algorithm: String,
    pub(in crate::v2) checksum: String,
    pub(in crate::v2) created_at_ms: i64,
    pub(in crate::v2) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_screen_snapshots)]
pub(in crate::v2) struct NewScreenSnapshotRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) session_id: String,
    pub(in crate::v2) pane_id: String,
    pub(in crate::v2) commit_id: String,
    pub(in crate::v2) projection_source: String,
    pub(in crate::v2) buffer_kind: String,
    pub(in crate::v2) rows: i32,
    pub(in crate::v2) cols: i32,
    pub(in crate::v2) base_event_seq: i64,
    pub(in crate::v2) high_water_event_seq: i64,
    pub(in crate::v2) high_water_byte_seq: Option<i64>,
    pub(in crate::v2) screen_json: String,
    pub(in crate::v2) parser_version: String,
    pub(in crate::v2) projection_version: String,
    pub(in crate::v2) checksum_algorithm: String,
    pub(in crate::v2) checksum: String,
    pub(in crate::v2) created_at_ms: i64,
    pub(in crate::v2) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = terminal_topology_snapshots)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[allow(dead_code)]
pub(in crate::v2) struct TopologySnapshotRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) session_id: String,
    pub(in crate::v2) commit_id: String,
    pub(in crate::v2) high_water_commit_seq: i64,
    pub(in crate::v2) pane_high_water_json: String,
    pub(in crate::v2) topology_json: String,
    pub(in crate::v2) payload_schema_id: Option<String>,
    pub(in crate::v2) checksum_algorithm: String,
    pub(in crate::v2) checksum: String,
    pub(in crate::v2) source: String,
    pub(in crate::v2) created_at_ms: i64,
    pub(in crate::v2) metadata_json: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = terminal_topology_snapshots)]
pub(in crate::v2) struct NewTopologySnapshotRow {
    pub(in crate::v2) id: String,
    pub(in crate::v2) session_id: String,
    pub(in crate::v2) commit_id: String,
    pub(in crate::v2) high_water_commit_seq: i64,
    pub(in crate::v2) pane_high_water_json: String,
    pub(in crate::v2) topology_json: String,
    pub(in crate::v2) payload_schema_id: Option<String>,
    pub(in crate::v2) checksum_algorithm: String,
    pub(in crate::v2) checksum: String,
    pub(in crate::v2) source: String,
    pub(in crate::v2) created_at_ms: i64,
    pub(in crate::v2) metadata_json: Option<String>,
}
