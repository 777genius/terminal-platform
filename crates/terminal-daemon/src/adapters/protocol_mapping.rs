use terminal_backend_api::{BackendError, BackendErrorKind, BackendSessionSummary};
use terminal_domain::{PaneId, SessionId, saved_session_compatibility};
use terminal_persistence::{
    CommandHistoryEntryRecord, HistoryGapRecord, PaneHistoryHydrationRecord,
    PaneHistoryReplayStrategy as PersistencePaneHistoryReplayStrategy,
    RestoreGuaranteeLevel as PersistenceRestoreGuaranteeLevel, RestorePlan, ScreenSnapshotRecord,
    StreamSegmentRecord,
};
use terminal_protocol::{
    CommandHistoryEntry, CommandHistoryResponse, HistoryReplayState, PaneHistoryGap,
    PaneHistoryReplayStrategy, PaneHistoryResponse, PaneHistoryRestoreEvidence,
    PaneHistoryRestorePlan, PaneHistoryScreenSnapshot, PaneHistorySegment, ProtocolError,
    RestoreGuaranteeLevel, RestoreSavedSessionResponse, SavedSessionRecord,
    SavedSessionRestoreSemantics, SavedSessionRestoreSemanticsV2, SavedSessionSummary,
};
use uuid::Uuid;

use crate::application::{RuntimeSavedSessionRecord, RuntimeSavedSessionSummary};

pub fn map_backend_error(error: BackendError) -> ProtocolError {
    let code = match error.kind {
        BackendErrorKind::Unsupported => "backend_unsupported",
        BackendErrorKind::NotFound => "backend_not_found",
        BackendErrorKind::InvalidInput => "backend_invalid_input",
        BackendErrorKind::Transport => "backend_transport",
        BackendErrorKind::Internal => "backend_internal",
    };
    let message = error.to_string();

    match error.degraded_reason {
        Some(degraded_reason) => {
            ProtocolError::with_degraded_reason(code, message, degraded_reason)
        }
        None => ProtocolError::new(code, message),
    }
}

pub fn map_saved_session_summary(session: RuntimeSavedSessionSummary) -> SavedSessionSummary {
    let compatibility = saved_session_compatibility(&session.manifest);
    let restore_semantics =
        saved_session_restore_semantics(session.has_launch, session.restore_plan.as_ref());
    let restore_semantics_v2 = saved_session_restore_semantics_v2(
        session.session_id,
        None,
        &restore_semantics,
        session.restore_plan.as_ref(),
    );

    SavedSessionSummary {
        session_id: session.session_id,
        route: session.route,
        title: session.title,
        saved_at_ms: session.saved_at_ms,
        manifest: session.manifest,
        compatibility,
        has_launch: session.has_launch,
        tab_count: session.tab_count,
        pane_count: session.pane_count,
        restore_semantics,
        restore_semantics_v2,
    }
}

pub fn map_saved_session_record(session: RuntimeSavedSessionRecord) -> SavedSessionRecord {
    let has_launch = session.launch.is_some();
    let compatibility = saved_session_compatibility(&session.manifest);
    let restore_semantics =
        saved_session_restore_semantics(has_launch, session.restore_plan.as_ref());
    let restore_semantics_v2 = saved_session_restore_semantics_v2(
        session.session_id,
        None,
        &restore_semantics,
        session.restore_plan.as_ref(),
    );

    SavedSessionRecord {
        session_id: session.session_id,
        route: session.route,
        title: session.title,
        launch: session.launch,
        manifest: session.manifest,
        compatibility,
        topology: session.topology,
        screens: session.screens,
        saved_at_ms: session.saved_at_ms,
        restore_semantics,
        restore_semantics_v2,
    }
}

pub fn map_restore_saved_session_response(
    saved_session_id: SessionId,
    saved_session: &RuntimeSavedSessionRecord,
    restored_session: BackendSessionSummary,
) -> RestoreSavedSessionResponse {
    let restore_semantics = saved_session_restore_semantics(
        saved_session.launch.is_some(),
        saved_session.restore_plan.as_ref(),
    );
    let restore_semantics_v2 = saved_session_restore_semantics_v2(
        saved_session_id,
        Some(restored_session.session_id),
        &restore_semantics,
        saved_session.restore_plan.as_ref(),
    );
    RestoreSavedSessionResponse {
        saved_session_id,
        manifest: saved_session.manifest.clone(),
        compatibility: saved_session_compatibility(&saved_session.manifest),
        session: restored_session,
        restore_semantics,
        restore_semantics_v2,
    }
}

fn saved_session_restore_semantics(
    has_launch: bool,
    restore_plan: Option<&RestorePlan>,
) -> SavedSessionRestoreSemantics {
    SavedSessionRestoreSemantics {
        restores_topology: true,
        restores_focus_state: true,
        restores_tab_titles: true,
        uses_saved_launch_spec: has_launch,
        replays_saved_screen_buffers: restore_plan_proves_screen_replay(restore_plan),
        preserves_process_state: false,
    }
}

fn saved_session_restore_semantics_v2(
    source_session_id: SessionId,
    restored_session_id: Option<SessionId>,
    legacy: &SavedSessionRestoreSemantics,
    restore_plan: Option<&RestorePlan>,
) -> Option<SavedSessionRestoreSemanticsV2> {
    let restore_plan = restore_plan?;
    let has_known_gaps = restore_plan.evidence.iter().any(|evidence| {
        evidence.kind == "history_gap"
            || evidence.kind == "history_gap_count"
                && evidence.value.parse::<i64>().unwrap_or(0) > 0
    });
    Some(SavedSessionRestoreSemanticsV2 {
        restores_topology: legacy.restores_topology,
        restores_focus_state: legacy.restores_focus_state,
        restores_tab_titles: legacy.restores_tab_titles,
        uses_saved_launch_spec: legacy.uses_saved_launch_spec,
        replays_saved_screen_buffers: restore_plan_proves_screen_replay(Some(restore_plan)),
        preserves_process_state: legacy.preserves_process_state,
        restore_guarantee_level: map_restore_guarantee_level(&restore_plan.guarantee_level),
        history_replay_state: map_history_replay_state(restore_plan, has_known_gaps),
        source_session_id,
        restored_session_id,
        latest_restore_drill_status: restore_plan.latest_restore_drill_status.clone(),
        has_known_gaps,
        evidence_refs: restore_plan
            .evidence
            .iter()
            .map(|evidence| format!("{}:{}", evidence.kind, evidence.value))
            .collect(),
    })
}

fn restore_plan_proves_screen_replay(restore_plan: Option<&RestorePlan>) -> bool {
    let Some(restore_plan) = restore_plan else {
        return false;
    };
    let has_passing_drill = restore_plan.latest_restore_drill_status.as_deref() == Some("passed");
    let replayable_guarantee = matches!(
        restore_plan.guarantee_level,
        PersistenceRestoreGuaranteeLevel::RawStreamReplay
            | PersistenceRestoreGuaranteeLevel::BasicHistory
    );
    has_passing_drill && replayable_guarantee
}

fn map_restore_guarantee_level(value: &PersistenceRestoreGuaranteeLevel) -> RestoreGuaranteeLevel {
    match value {
        PersistenceRestoreGuaranteeLevel::RawStreamReplay => RestoreGuaranteeLevel::RichHistory,
        PersistenceRestoreGuaranteeLevel::BasicHistory => RestoreGuaranteeLevel::BasicHistory,
        PersistenceRestoreGuaranteeLevel::VisualSnapshotOnly => {
            RestoreGuaranteeLevel::VisualRestoreOnly
        }
        PersistenceRestoreGuaranteeLevel::LiveMuxAttach => RestoreGuaranteeLevel::BasicHistory,
        PersistenceRestoreGuaranteeLevel::DegradedHistory
        | PersistenceRestoreGuaranteeLevel::None => RestoreGuaranteeLevel::HistoryDegraded,
    }
}

fn map_history_replay_state(
    restore_plan: &RestorePlan,
    has_known_gaps: bool,
) -> HistoryReplayState {
    if has_known_gaps {
        return HistoryReplayState::PartiallyReplayedWithGaps;
    }
    match restore_plan.guarantee_level {
        PersistenceRestoreGuaranteeLevel::RawStreamReplay => {
            HistoryReplayState::ReplayedFromJournal
        }
        PersistenceRestoreGuaranteeLevel::BasicHistory => HistoryReplayState::ReplayedFromJournal,
        PersistenceRestoreGuaranteeLevel::VisualSnapshotOnly => {
            HistoryReplayState::HydratedFromSnapshot
        }
        PersistenceRestoreGuaranteeLevel::LiveMuxAttach => HistoryReplayState::HydratedFromSnapshot,
        PersistenceRestoreGuaranteeLevel::None => HistoryReplayState::NotAvailable,
        PersistenceRestoreGuaranteeLevel::DegradedHistory => {
            if restore_plan.latest_screen_snapshot_id.is_some() {
                HistoryReplayState::SnapshotOnly
            } else {
                HistoryReplayState::NotAvailable
            }
        }
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    fn restore_plan(
        guarantee_level: PersistenceRestoreGuaranteeLevel,
        latest_restore_drill_status: Option<&str>,
    ) -> RestorePlan {
        RestorePlan {
            session_id: Uuid::new_v4().to_string(),
            guarantee_level,
            latest_screen_snapshot_id: Some("screen-snapshot".to_string()),
            latest_topology_snapshot_id: Some("topology-snapshot".to_string()),
            high_water_commit_seq: 1,
            latest_restore_drill_status: latest_restore_drill_status.map(str::to_string),
            evidence: vec![terminal_persistence::RestoreEvidence {
                kind: "stream_segment_count".to_string(),
                value: "1".to_string(),
            }],
        }
    }

    #[test]
    fn legacy_snapshot_only_semantics_do_not_claim_screen_replay() {
        let source_session_id = SessionId::new();
        let plan =
            restore_plan(PersistenceRestoreGuaranteeLevel::VisualSnapshotOnly, Some("passed"));
        let legacy = saved_session_restore_semantics(true, Some(&plan));
        let v2 = saved_session_restore_semantics_v2(source_session_id, None, &legacy, Some(&plan))
            .expect("v2 semantics should map");

        assert!(!legacy.replays_saved_screen_buffers);
        assert!(!v2.replays_saved_screen_buffers);
        assert_eq!(v2.restore_guarantee_level, RestoreGuaranteeLevel::VisualRestoreOnly);
        assert_eq!(v2.history_replay_state, HistoryReplayState::HydratedFromSnapshot);
    }

    #[test]
    fn v2_basic_history_with_restore_drill_claims_screen_replay() {
        let source_session_id = SessionId::new();
        let plan = restore_plan(PersistenceRestoreGuaranteeLevel::BasicHistory, Some("passed"));
        let legacy = saved_session_restore_semantics(true, Some(&plan));
        let v2 = saved_session_restore_semantics_v2(source_session_id, None, &legacy, Some(&plan))
            .expect("v2 semantics should map");

        assert!(legacy.replays_saved_screen_buffers);
        assert!(v2.replays_saved_screen_buffers);
        assert_eq!(v2.restore_guarantee_level, RestoreGuaranteeLevel::BasicHistory);
        assert_eq!(v2.history_replay_state, HistoryReplayState::ReplayedFromJournal);
    }

    #[test]
    fn v2_basic_history_without_restore_drill_stays_conservative() {
        let plan = restore_plan(PersistenceRestoreGuaranteeLevel::BasicHistory, None);
        let legacy = saved_session_restore_semantics(true, Some(&plan));

        assert!(!legacy.replays_saved_screen_buffers);
        assert!(!restore_plan_proves_screen_replay(Some(&plan)));
    }
}

fn parse_session_id(value: &str) -> Result<SessionId, ProtocolError> {
    Uuid::parse_str(value)
        .map(SessionId::from)
        .map_err(|error| ProtocolError::new("invalid_persisted_session_id", error.to_string()))
}

fn parse_pane_id(value: &str) -> Result<PaneId, ProtocolError> {
    Uuid::parse_str(value)
        .map(PaneId::from)
        .map_err(|error| ProtocolError::new("invalid_persisted_pane_id", error.to_string()))
}
