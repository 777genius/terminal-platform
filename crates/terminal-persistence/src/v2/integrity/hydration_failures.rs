use super::super::*;
use super::*;

pub(in crate::v2) fn stream_segment_hydration_failure(row: &StreamSegmentRow) -> Option<String> {
    let mut failures = Vec::new();
    validate_stream_segment_ranges(row, &mut failures);
    validate_checksum_bytes(
        "stream_segment",
        &row.id,
        &row.payload,
        &row.checksum_algorithm,
        &row.checksum,
        &mut failures,
    );
    failures.into_iter().next()
}

pub(in crate::v2) fn persist_hydration_segment_failure(
    connection: &mut SqliteConnection,
    session_id: &str,
    row: &StreamSegmentRow,
    failure: &str,
    detected_at_ms: i64,
) -> Result<(), TerminalPersistenceV2Error> {
    let validation = HistoryValidation {
        journal_events_checked: 0,
        stream_segments_checked: 1,
        screen_snapshots_checked: 0,
        topology_snapshots_checked: 0,
        failures: vec![failure.to_string()],
    };
    persist_history_validation_health_records(
        connection,
        Some(session_id),
        &validation,
        detected_at_ms,
        Some("hydrate_pane_history"),
    )?;

    let event_range_valid = row.event_seq_low <= row.event_seq_high;
    let byte_range_valid = row.byte_low < row.byte_high;
    let existing_gap = if event_range_valid {
        terminal_history_gaps::table
            .filter(terminal_history_gaps::session_id.eq(&row.session_id))
            .filter(terminal_history_gaps::pane_id.eq(Some(row.pane_id.clone())))
            .filter(terminal_history_gaps::stream_id.eq(&row.stream_id))
            .filter(terminal_history_gaps::gap_kind.eq("corrupted_segment"))
            .filter(terminal_history_gaps::event_seq_low.eq(Some(row.event_seq_low)))
            .filter(terminal_history_gaps::event_seq_high.eq(Some(row.event_seq_high)))
            .select(terminal_history_gaps::id)
            .first::<String>(connection)
            .optional()?
    } else {
        None
    };
    if existing_gap.is_some() {
        return Ok(());
    }

    let metadata_json = Some(serde_json::to_string(&serde_json::json!({
        "stream_segment_id": row.id,
        "failure": failure,
        "detected_by": "hydrate_pane_history"
    }))?);
    let gap = NewHistoryGapRow {
        id: new_id(),
        session_id: row.session_id.clone(),
        pane_id: Some(row.pane_id.clone()),
        stream_id: row.stream_id.clone(),
        gap_kind: "corrupted_segment".to_string(),
        event_seq_low: event_range_valid.then_some(row.event_seq_low),
        event_seq_high: event_range_valid.then_some(row.event_seq_high),
        byte_low: byte_range_valid.then_some(row.byte_low),
        byte_high: byte_range_valid.then_some(row.byte_high),
        estimated_dropped_bytes: byte_range_valid.then_some(row.byte_high - row.byte_low),
        estimated_dropped_events: event_range_valid
            .then_some(row.event_seq_high - row.event_seq_low + 1),
        reason: "canonical stream segment failed hydration validation".to_string(),
        writer_generation: Some(row.writer_generation.clone()),
        opened_at_ms: detected_at_ms,
        closed_at_ms: Some(detected_at_ms),
        metadata_json,
    };
    insert_into(terminal_history_gaps::table).values(&gap).execute(connection)?;
    Ok(())
}
