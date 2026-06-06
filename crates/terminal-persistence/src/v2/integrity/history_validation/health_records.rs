use super::super::super::*;

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
