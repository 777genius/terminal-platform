use super::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum NodePaneHistoryReplayStrategy {
    Empty,
    RawVtStream,
    RenderedSnapshot,
    Mixed,
    Degraded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodePaneHistoryRestoreEvidence {
    pub kind: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodePaneHistoryRestorePlan {
    pub session_id: String,
    pub restore_guarantee_level: String,
    pub latest_screen_snapshot_id: Option<String>,
    pub latest_topology_snapshot_id: Option<String>,
    pub high_water_commit_seq: i64,
    pub latest_restore_drill_status: Option<String>,
    pub evidence: Vec<NodePaneHistoryRestoreEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodePaneHistoryScreenSnapshot {
    pub id: String,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodePaneHistorySegment {
    pub id: String,
    pub event_seq_low: i64,
    pub event_seq_high: i64,
    pub byte_low: i64,
    pub byte_high: i64,
    pub payload: Vec<u8>,
    pub checksum: String,
    pub capture_semantics: String,
    pub created_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodePaneHistoryGap {
    pub id: String,
    pub pane_id: Option<String>,
    pub stream_id: String,
    pub gap_kind: String,
    pub event_seq_low: Option<i64>,
    pub event_seq_high: Option<i64>,
    pub byte_low: Option<i64>,
    pub byte_high: Option<i64>,
    pub estimated_dropped_bytes: Option<i64>,
    pub estimated_dropped_events: Option<i64>,
    pub reason: String,
    pub opened_at_ms: i64,
    pub closed_at_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodePaneHistory {
    pub session_id: String,
    pub pane_id: String,
    pub from_event_seq: i64,
    pub max_segments: i64,
    pub max_bytes: i64,
    pub restore_plan: NodePaneHistoryRestorePlan,
    pub latest_screen_snapshot: Option<NodePaneHistoryScreenSnapshot>,
    pub segments: Vec<NodePaneHistorySegment>,
    pub gaps: Vec<NodePaneHistoryGap>,
    pub replay_strategy: NodePaneHistoryReplayStrategy,
    pub has_more_segments: bool,
    pub next_event_seq: Option<i64>,
    pub total_payload_bytes: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeCommandHistoryEntry {
    pub id: String,
    pub session_id: Option<String>,
    pub pane_id: Option<String>,
    pub display_text: String,
    pub last_used_at_ms: i64,
    pub use_count: i64,
}
