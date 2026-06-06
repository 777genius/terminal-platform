use super::super::super::*;
use super::super::*;

pub(in crate::v2) fn validate_history_checksums(
    connection: &mut SqliteConnection,
    session_id: Option<&str>,
) -> Result<HistoryValidation, TerminalPersistenceV2Error> {
    let mut failures = Vec::new();
    let schema_ids = terminal_payload_schemas::table
        .select(terminal_payload_schemas::id)
        .load::<String>(connection)?;

    let mut journal_query = terminal_journal_events::table.into_boxed();
    if let Some(session_id) = session_id {
        journal_query = journal_query.filter(terminal_journal_events::session_id.eq(session_id));
    }
    let journal_rows = journal_query
        .select((
            terminal_journal_events::id,
            terminal_journal_events::payload_json,
            terminal_journal_events::payload_schema_id,
        ))
        .load::<(String, Option<String>, Option<String>)>(connection)?;
    for (id, payload_json, payload_schema_id) in &journal_rows {
        validate_payload_schema_ref(
            "journal_event",
            id,
            payload_json.is_some(),
            payload_schema_id.as_deref(),
            &schema_ids,
            &mut failures,
        );
    }

    let mut segment_query = terminal_stream_segments::table.into_boxed();
    if let Some(session_id) = session_id {
        segment_query = segment_query.filter(terminal_stream_segments::session_id.eq(session_id));
    }
    let segment_rows =
        segment_query.select(StreamSegmentRow::as_select()).load::<StreamSegmentRow>(connection)?;
    for row in &segment_rows {
        validate_stream_segment_ranges(row, &mut failures);
        validate_checksum_bytes(
            "stream_segment",
            &row.id,
            &row.payload,
            &row.checksum_algorithm,
            &row.checksum,
            &mut failures,
        );
    }

    let mut screen_query = terminal_screen_snapshots::table.into_boxed();
    if let Some(session_id) = session_id {
        screen_query = screen_query.filter(terminal_screen_snapshots::session_id.eq(session_id));
    }
    let screen_rows = screen_query
        .select((
            terminal_screen_snapshots::id,
            terminal_screen_snapshots::screen_json,
            terminal_screen_snapshots::checksum_algorithm,
            terminal_screen_snapshots::checksum,
        ))
        .load::<(String, String, String, String)>(connection)?;
    for (id, payload, algorithm, checksum) in &screen_rows {
        validate_checksum_text("screen_snapshot", id, payload, algorithm, checksum, &mut failures);
    }

    let mut topology_query = terminal_topology_snapshots::table.into_boxed();
    if let Some(session_id) = session_id {
        topology_query =
            topology_query.filter(terminal_topology_snapshots::session_id.eq(session_id));
    }
    let topology_rows = topology_query
        .select((
            terminal_topology_snapshots::id,
            terminal_topology_snapshots::pane_high_water_json,
            terminal_topology_snapshots::topology_json,
            terminal_topology_snapshots::payload_schema_id,
            terminal_topology_snapshots::checksum_algorithm,
            terminal_topology_snapshots::checksum,
        ))
        .load::<(String, String, String, Option<String>, String, String)>(connection)?;
    for (id, pane_high_water_json, payload, payload_schema_id, algorithm, checksum) in
        &topology_rows
    {
        validate_payload_schema_ref(
            "topology_snapshot",
            id,
            true,
            payload_schema_id.as_deref(),
            &schema_ids,
            &mut failures,
        );
        validate_checksum_text(
            "topology_snapshot",
            id,
            payload,
            algorithm,
            checksum,
            &mut failures,
        );
        validate_topology_pane_high_water_json_payload(id, pane_high_water_json, &mut failures);
    }

    validate_sequence_invariants(connection, session_id, &mut failures)?;

    Ok(HistoryValidation {
        journal_events_checked: journal_rows.len(),
        stream_segments_checked: segment_rows.len(),
        screen_snapshots_checked: screen_rows.len(),
        topology_snapshots_checked: topology_rows.len(),
        failures,
    })
}
