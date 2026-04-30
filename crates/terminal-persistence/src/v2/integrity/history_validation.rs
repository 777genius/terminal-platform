use super::super::*;
use super::*;

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

pub(in crate::v2) fn persist_history_validation_health_records(
    connection: &mut SqliteConnection,
    session_id: Option<&str>,
    validation: &HistoryValidation,
    detected_at_ms: i64,
    evidence_ref: Option<&str>,
) -> Result<(), TerminalPersistenceV2Error> {
    for failure in &validation.failures {
        let detection_kind = if failure.contains("checksum mismatch") {
            "checksum_mismatch"
        } else if failure.contains("payload_schema_id") {
            "migration_mismatch"
        } else if failure.contains("topology high-water")
            || failure.contains("topology high_water_event_seq")
            || failure.contains("pane_high_water_json")
        {
            "projection_drift"
        } else if failure.starts_with("stream_cursor:")
            || failure.starts_with("pane:")
            || failure.starts_with("session_cursor:")
            || failure.starts_with("commit_log:")
            || failure.starts_with("stream_segment:")
        {
            "missing_segment"
        } else {
            "manual"
        };
        let is_canonical_replay_source =
            failure.starts_with("stream_segment:") || failure.starts_with("journal_event:");
        let severity = if is_canonical_replay_source { "critical" } else { "error" };
        let action_state =
            if is_canonical_replay_source { "quarantined" } else { "rebuild_pending" };
        let affected_ref = Some(failure.clone());
        let existing = terminal_data_health_records::table
            .filter(terminal_data_health_records::affected_ref.eq(affected_ref.clone()))
            .filter(terminal_data_health_records::detection_kind.eq(detection_kind))
            .filter(terminal_data_health_records::action_state.ne("resolved"))
            .filter(terminal_data_health_records::action_state.ne("ignored"))
            .select(DataHealthRecordRow::as_select())
            .first::<DataHealthRecordRow>(connection)
            .optional()?;
        if existing.is_some() {
            continue;
        }

        let details_json = Some(serde_json::to_string(&serde_json::json!({
            "failure": failure,
            "evidence_ref": evidence_ref,
            "validation": {
                "journal_events_checked": validation.journal_events_checked,
                "stream_segments_checked": validation.stream_segments_checked,
                "screen_snapshots_checked": validation.screen_snapshots_checked,
                "topology_snapshots_checked": validation.topology_snapshots_checked
            }
        }))?);
        let row = NewDataHealthRecordRow {
            id: new_id(),
            session_id: session_id.map(ToOwned::to_owned),
            pane_id: None,
            detection_kind: detection_kind.to_string(),
            severity: severity.to_string(),
            first_bad_event_seq: None,
            affected_ref,
            action_state: action_state.to_string(),
            detected_at_ms,
            resolved_at_ms: None,
            details_json,
            metadata_json: None,
        };
        insert_into(terminal_data_health_records::table).values(&row).execute(connection)?;
    }
    Ok(())
}
