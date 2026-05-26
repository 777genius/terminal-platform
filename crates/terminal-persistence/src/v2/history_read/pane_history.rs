use super::{
    super::*,
    gaps::{has_history_gap_at_or_after, load_history_gaps},
    limits::PaneHistoryLimits,
    segments::{collect_hydratable_segments, load_stream_segment_rows},
};

impl TerminalPersistenceV2 {
    pub fn hydrate_pane_history(
        &self,
        session_id: &str,
        pane_id: &str,
        from_event_seq: Option<i64>,
        max_segments: Option<i64>,
        max_bytes: Option<i64>,
    ) -> Result<PaneHistoryHydrationRecord, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        self.hydrate_pane_history_with_connection(
            &mut connection,
            session_id,
            pane_id,
            from_event_seq,
            max_segments,
            max_bytes,
        )
    }

    pub(crate) fn hydrate_pane_history_with_connection(
        &self,
        connection: &mut SqliteConnection,
        session_id: &str,
        pane_id: &str,
        from_event_seq: Option<i64>,
        max_segments: Option<i64>,
        max_bytes: Option<i64>,
    ) -> Result<PaneHistoryHydrationRecord, TerminalPersistenceV2Error> {
        let limits = PaneHistoryLimits::from_inputs(from_event_seq, max_segments, max_bytes);
        let now = self.config.clock.now_ms();

        let latest_topology = load_latest_valid_topology_snapshot(
            connection,
            session_id,
            now,
            "hydrate_pane_history",
        )?;
        let topology_pane_high_water = latest_topology
            .as_ref()
            .map(|topology| parse_pane_high_water_json(&topology.pane_high_water_json))
            .transpose()?;
        let latest_screen_snapshot = load_latest_valid_screen_snapshot(
            connection,
            session_id,
            Some(pane_id),
            topology_pane_high_water.as_ref(),
            now,
            "hydrate_pane_history",
        )?
        .map(ScreenSnapshotRecord::from);

        let fetched_segments = load_stream_segment_rows(
            connection,
            session_id,
            pane_id,
            limits.from_event_seq,
            limits.max_segments,
        )?;
        let hydrated_segments = collect_hydratable_segments(
            connection,
            session_id,
            pane_id,
            fetched_segments,
            limits.max_segments,
            limits.max_bytes,
            now,
        )?;
        let page_event_seq_high = hydrated_segments.next_event_seq.map(|event_seq| event_seq - 1);
        let gaps = load_history_gaps(
            connection,
            session_id,
            pane_id,
            limits.from_event_seq,
            page_event_seq_high,
        )?;
        let has_more_segments = if hydrated_segments.has_more_segments {
            true
        } else {
            hydrated_segments
                .next_event_seq
                .map(|event_seq| {
                    has_history_gap_at_or_after(connection, session_id, pane_id, event_seq)
                })
                .transpose()?
                .unwrap_or(false)
        };
        let restore_plan = self.restore_plan_with_connection(connection, session_id)?;
        let replay_strategy = PaneHistoryReplayStrategy::from_evidence(
            &hydrated_segments.segments,
            latest_screen_snapshot.as_ref(),
            &gaps,
        );

        Ok(PaneHistoryHydrationRecord {
            session_id: session_id.to_string(),
            pane_id: pane_id.to_string(),
            from_event_seq: limits.from_event_seq,
            max_segments: limits.max_segments,
            max_bytes: limits.max_bytes,
            restore_plan,
            latest_screen_snapshot,
            segments: hydrated_segments.segments,
            gaps,
            replay_strategy,
            has_more_segments,
            next_event_seq: hydrated_segments.next_event_seq,
            total_payload_bytes: hydrated_segments.total_payload_bytes,
        })
    }
}
