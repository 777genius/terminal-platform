use serde::{Deserialize, Serialize};

use super::{HistoryGapRecord, RestorePlan, ScreenSnapshotRecord};
use crate::v2::StreamSegmentRecord;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaneHistoryReplayStrategy {
    Empty,
    RawVtStream,
    RenderedSnapshot,
    Mixed,
    Degraded,
}

impl PaneHistoryReplayStrategy {
    #[must_use]
    pub(crate) fn from_evidence(
        segments: &[StreamSegmentRecord],
        latest_screen_snapshot: Option<&ScreenSnapshotRecord>,
        gaps: &[HistoryGapRecord],
    ) -> Self {
        if !gaps.is_empty() {
            return Self::Degraded;
        }
        let has_raw = segments.iter().any(|segment| segment.capture_semantics == "raw_vt_stream");
        let has_rendered =
            segments.iter().any(|segment| segment.capture_semantics != "raw_vt_stream")
                || latest_screen_snapshot.is_some();
        match (has_raw, has_rendered) {
            (true, false) => Self::RawVtStream,
            (false, true) => Self::RenderedSnapshot,
            (true, true) => Self::Mixed,
            (false, false) => Self::Empty,
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::RawVtStream => "raw_vt_stream",
            Self::RenderedSnapshot => "rendered_snapshot",
            Self::Mixed => "mixed",
            Self::Degraded => "degraded",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaneHistoryHydrationRecord {
    pub session_id: String,
    pub pane_id: String,
    pub from_event_seq: i64,
    pub max_segments: i64,
    pub max_bytes: i64,
    pub restore_plan: RestorePlan,
    pub latest_screen_snapshot: Option<ScreenSnapshotRecord>,
    pub segments: Vec<StreamSegmentRecord>,
    pub gaps: Vec<HistoryGapRecord>,
    pub replay_strategy: PaneHistoryReplayStrategy,
    pub has_more_segments: bool,
    pub next_event_seq: Option<i64>,
    pub total_payload_bytes: i64,
}
