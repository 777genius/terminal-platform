use super::{prelude::*, *};

impl From<&PaneHistoryReplayStrategy> for NodePaneHistoryReplayStrategy {
    fn from(value: &PaneHistoryReplayStrategy) -> Self {
        match value {
            PaneHistoryReplayStrategy::Empty => Self::Empty,
            PaneHistoryReplayStrategy::RawVtStream => Self::RawVtStream,
            PaneHistoryReplayStrategy::RenderedSnapshot => Self::RenderedSnapshot,
            PaneHistoryReplayStrategy::Mixed => Self::Mixed,
            PaneHistoryReplayStrategy::Degraded => Self::Degraded,
        }
    }
}

impl From<&PaneHistoryRestoreEvidence> for NodePaneHistoryRestoreEvidence {
    fn from(value: &PaneHistoryRestoreEvidence) -> Self {
        Self { kind: value.kind.clone(), value: value.value.clone() }
    }
}

impl From<&PaneHistoryRestorePlan> for NodePaneHistoryRestorePlan {
    fn from(value: &PaneHistoryRestorePlan) -> Self {
        Self {
            session_id: value.session_id.0.to_string(),
            restore_guarantee_level: value.restore_guarantee_level.clone(),
            latest_screen_snapshot_id: value.latest_screen_snapshot_id.clone(),
            latest_topology_snapshot_id: value.latest_topology_snapshot_id.clone(),
            high_water_commit_seq: value.high_water_commit_seq,
            latest_restore_drill_status: value.latest_restore_drill_status.clone(),
            evidence: value.evidence.iter().map(Into::into).collect(),
        }
    }
}

impl From<&PaneHistoryScreenSnapshot> for NodePaneHistoryScreenSnapshot {
    fn from(value: &PaneHistoryScreenSnapshot) -> Self {
        Self {
            id: value.id.clone(),
            pane_id: value.pane_id.0.to_string(),
            projection_source: value.projection_source.clone(),
            buffer_kind: value.buffer_kind.clone(),
            rows: value.rows,
            cols: value.cols,
            base_event_seq: value.base_event_seq,
            high_water_event_seq: value.high_water_event_seq,
            high_water_byte_seq: value.high_water_byte_seq,
            screen_json: value.screen_json.clone(),
            parser_version: value.parser_version.clone(),
            projection_version: value.projection_version.clone(),
            checksum: value.checksum.clone(),
            created_at_ms: value.created_at_ms,
        }
    }
}

impl From<&PaneHistorySegment> for NodePaneHistorySegment {
    fn from(value: &PaneHistorySegment) -> Self {
        Self {
            id: value.id.clone(),
            event_seq_low: value.event_seq_low,
            event_seq_high: value.event_seq_high,
            byte_low: value.byte_low,
            byte_high: value.byte_high,
            payload: value.payload.clone(),
            checksum: value.checksum.clone(),
            capture_semantics: value.capture_semantics.clone(),
            created_at_ms: value.created_at_ms,
        }
    }
}

impl From<&PaneHistoryGap> for NodePaneHistoryGap {
    fn from(value: &PaneHistoryGap) -> Self {
        Self {
            id: value.id.clone(),
            pane_id: value.pane_id.map(|pane_id| pane_id.0.to_string()),
            stream_id: value.stream_id.clone(),
            gap_kind: value.gap_kind.clone(),
            event_seq_low: value.event_seq_low,
            event_seq_high: value.event_seq_high,
            byte_low: value.byte_low,
            byte_high: value.byte_high,
            estimated_dropped_bytes: value.estimated_dropped_bytes,
            estimated_dropped_events: value.estimated_dropped_events,
            reason: value.reason.clone(),
            opened_at_ms: value.opened_at_ms,
            closed_at_ms: value.closed_at_ms,
        }
    }
}

impl From<&PaneHistoryResponse> for NodePaneHistory {
    fn from(value: &PaneHistoryResponse) -> Self {
        Self {
            session_id: value.session_id.0.to_string(),
            pane_id: value.pane_id.0.to_string(),
            from_event_seq: value.from_event_seq,
            max_segments: value.max_segments,
            max_bytes: value.max_bytes,
            restore_plan: (&value.restore_plan).into(),
            latest_screen_snapshot: value.latest_screen_snapshot.as_ref().map(Into::into),
            segments: value.segments.iter().map(Into::into).collect(),
            gaps: value.gaps.iter().map(Into::into).collect(),
            replay_strategy: (&value.replay_strategy).into(),
            has_more_segments: value.has_more_segments,
            next_event_seq: value.next_event_seq,
            total_payload_bytes: value.total_payload_bytes,
        }
    }
}

impl From<&CommandHistoryEntry> for NodeCommandHistoryEntry {
    fn from(value: &CommandHistoryEntry) -> Self {
        Self {
            id: value.id.clone(),
            session_id: value.session_id.map(|session_id| session_id.0.to_string()),
            pane_id: value.pane_id.map(|pane_id| pane_id.0.to_string()),
            display_text: value.display_text.clone(),
            last_used_at_ms: value.last_used_at_ms,
            use_count: value.use_count,
        }
    }
}
