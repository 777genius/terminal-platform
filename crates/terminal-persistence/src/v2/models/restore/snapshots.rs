use serde::{Deserialize, Serialize};

use crate::v2::rows::ScreenSnapshotRow;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenSnapshotRecord {
    pub id: String,
    pub session_id: String,
    pub pane_id: String,
    pub projection_source: String,
    pub buffer_kind: String,
    pub rows: i32,
    pub cols: i32,
    pub base_event_seq: i64,
    pub high_water_event_seq: i64,
    pub high_water_byte_seq: Option<i64>,
    pub screen_json: String,
    pub parser_version: String,
    pub projection_version: String,
    pub checksum: String,
    pub created_at_ms: i64,
}

impl From<ScreenSnapshotRow> for ScreenSnapshotRecord {
    fn from(row: ScreenSnapshotRow) -> Self {
        Self {
            id: row.id,
            session_id: row.session_id,
            pane_id: row.pane_id,
            projection_source: row.projection_source,
            buffer_kind: row.buffer_kind,
            rows: row.rows,
            cols: row.cols,
            base_event_seq: row.base_event_seq,
            high_water_event_seq: row.high_water_event_seq,
            high_water_byte_seq: row.high_water_byte_seq,
            screen_json: row.screen_json,
            parser_version: row.parser_version,
            projection_version: row.projection_version,
            checksum: row.checksum,
            created_at_ms: row.created_at_ms,
        }
    }
}
