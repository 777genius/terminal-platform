use serde::{Deserialize, Serialize};

use crate::v2::rows::HistoryGapRow;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryGapRecord {
    pub id: String,
    pub session_id: String,
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

impl From<HistoryGapRow> for HistoryGapRecord {
    fn from(row: HistoryGapRow) -> Self {
        Self {
            id: row.id,
            session_id: row.session_id,
            pane_id: row.pane_id,
            stream_id: row.stream_id,
            gap_kind: row.gap_kind,
            event_seq_low: row.event_seq_low,
            event_seq_high: row.event_seq_high,
            byte_low: row.byte_low,
            byte_high: row.byte_high,
            estimated_dropped_bytes: row.estimated_dropped_bytes,
            estimated_dropped_events: row.estimated_dropped_events,
            reason: row.reason,
            opened_at_ms: row.opened_at_ms,
            closed_at_ms: row.closed_at_ms,
        }
    }
}
