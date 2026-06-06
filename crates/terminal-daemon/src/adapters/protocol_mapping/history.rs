use terminal_persistence::{
    CommandHistoryEntryRecord, HistoryGapRecord, PaneHistoryHydrationRecord,
    PaneHistoryReplayStrategy as PersistencePaneHistoryReplayStrategy, RestorePlan,
    ScreenSnapshotRecord, StreamSegmentRecord,
};
use terminal_protocol::{
    CommandHistoryEntry, CommandHistoryResponse, PaneHistoryGap, PaneHistoryReplayStrategy,
    PaneHistoryResponse, PaneHistoryRestoreEvidence, PaneHistoryRestorePlan,
    PaneHistoryScreenSnapshot, PaneHistorySegment, ProtocolError,
};

use super::ids::{parse_pane_id, parse_session_id};

pub fn map_pane_history(
    record: PaneHistoryHydrationRecord,
) -> Result<PaneHistoryResponse, ProtocolError> {
    let session_id = parse_session_id(&record.session_id)?;
    let pane_id = parse_pane_id(&record.pane_id)?;
    Ok(PaneHistoryResponse {
        session_id,
        pane_id,
        from_event_seq: record.from_event_seq,
        max_segments: record.max_segments,
        max_bytes: record.max_bytes,
        restore_plan: map_restore_plan(record.restore_plan)?,
        latest_screen_snapshot: record
            .latest_screen_snapshot
            .map(map_screen_snapshot_record)
            .transpose()?,
        segments: record.segments.into_iter().map(map_stream_segment).collect(),
        gaps: record.gaps.into_iter().map(map_history_gap).collect::<Result<Vec<_>, _>>()?,
        replay_strategy: map_replay_strategy(record.replay_strategy),
        has_more_segments: record.has_more_segments,
        next_event_seq: record.next_event_seq,
        total_payload_bytes: record.total_payload_bytes,
    })
}

pub fn map_command_history(
    entries: Vec<CommandHistoryEntryRecord>,
) -> Result<CommandHistoryResponse, ProtocolError> {
    Ok(CommandHistoryResponse {
        entries: entries
            .into_iter()
            .map(|entry| {
                Ok(CommandHistoryEntry {
                    id: entry.id,
                    session_id: entry.session_id.as_deref().map(parse_session_id).transpose()?,
                    pane_id: entry.pane_id.as_deref().map(parse_pane_id).transpose()?,
                    display_text: entry.display_text,
                    last_used_at_ms: entry.last_used_at_ms,
                    use_count: entry.use_count,
                })
            })
            .collect::<Result<Vec<_>, ProtocolError>>()?,
    })
}

fn map_restore_plan(plan: RestorePlan) -> Result<PaneHistoryRestorePlan, ProtocolError> {
    Ok(PaneHistoryRestorePlan {
        session_id: parse_session_id(&plan.session_id)?,
        restore_guarantee_level: plan.guarantee_level.as_str().to_string(),
        latest_screen_snapshot_id: plan.latest_screen_snapshot_id,
        latest_topology_snapshot_id: plan.latest_topology_snapshot_id,
        high_water_commit_seq: plan.high_water_commit_seq,
        latest_restore_drill_status: plan.latest_restore_drill_status,
        evidence: plan
            .evidence
            .into_iter()
            .map(|evidence| PaneHistoryRestoreEvidence {
                kind: evidence.kind,
                value: evidence.value,
            })
            .collect(),
    })
}

fn map_screen_snapshot_record(
    snapshot: ScreenSnapshotRecord,
) -> Result<PaneHistoryScreenSnapshot, ProtocolError> {
    Ok(PaneHistoryScreenSnapshot {
        id: snapshot.id,
        pane_id: parse_pane_id(&snapshot.pane_id)?,
        projection_source: snapshot.projection_source,
        buffer_kind: snapshot.buffer_kind,
        rows: snapshot.rows,
        cols: snapshot.cols,
        base_event_seq: snapshot.base_event_seq,
        high_water_event_seq: snapshot.high_water_event_seq,
        high_water_byte_seq: snapshot.high_water_byte_seq,
        screen_json: snapshot.screen_json,
        parser_version: snapshot.parser_version,
        projection_version: snapshot.projection_version,
        checksum: snapshot.checksum,
        created_at_ms: snapshot.created_at_ms,
    })
}

fn map_stream_segment(segment: StreamSegmentRecord) -> PaneHistorySegment {
    PaneHistorySegment {
        id: segment.id,
        event_seq_low: segment.event_seq_low,
        event_seq_high: segment.event_seq_high,
        byte_low: segment.byte_low,
        byte_high: segment.byte_high,
        payload: segment.payload,
        checksum: segment.checksum,
        capture_semantics: segment.capture_semantics,
        created_at_ms: segment.created_at_ms,
    }
}

fn map_history_gap(gap: HistoryGapRecord) -> Result<PaneHistoryGap, ProtocolError> {
    Ok(PaneHistoryGap {
        id: gap.id,
        pane_id: gap.pane_id.as_deref().map(parse_pane_id).transpose()?,
        stream_id: gap.stream_id,
        gap_kind: gap.gap_kind,
        event_seq_low: gap.event_seq_low,
        event_seq_high: gap.event_seq_high,
        byte_low: gap.byte_low,
        byte_high: gap.byte_high,
        estimated_dropped_bytes: gap.estimated_dropped_bytes,
        estimated_dropped_events: gap.estimated_dropped_events,
        reason: gap.reason,
        opened_at_ms: gap.opened_at_ms,
        closed_at_ms: gap.closed_at_ms,
    })
}

fn map_replay_strategy(
    strategy: PersistencePaneHistoryReplayStrategy,
) -> PaneHistoryReplayStrategy {
    match strategy {
        PersistencePaneHistoryReplayStrategy::Empty => PaneHistoryReplayStrategy::Empty,
        PersistencePaneHistoryReplayStrategy::RawVtStream => PaneHistoryReplayStrategy::RawVtStream,
        PersistencePaneHistoryReplayStrategy::RenderedSnapshot => {
            PaneHistoryReplayStrategy::RenderedSnapshot
        }
        PersistencePaneHistoryReplayStrategy::Mixed => PaneHistoryReplayStrategy::Mixed,
        PersistencePaneHistoryReplayStrategy::Degraded => PaneHistoryReplayStrategy::Degraded,
    }
}
