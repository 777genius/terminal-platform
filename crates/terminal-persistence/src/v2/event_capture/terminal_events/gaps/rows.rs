use super::{super::super::super::*, transaction::HistoryGapTransaction};

pub(super) fn insert_history_gap_journal_event(
    connection: &mut SqliteConnection,
    input: &HistoryGapTransaction<'_>,
    commit_id: &str,
    event_seq: i64,
) -> Result<String, TerminalPersistenceV2Error> {
    let scope = event_scope(input.session_id, Some(input.pane_id));
    let event_id = new_id();
    let event = NewJournalEventRow {
        id: event_id.clone(),
        session_id: input.session_id.to_string(),
        pane_id: Some(input.pane_id.to_string()),
        commit_id: commit_id.to_string(),
        stream_id: input.stream_id.to_string(),
        event_scope_kind: scope.kind,
        event_scope_id: scope.id,
        event_seq,
        event_type: "history_gap".to_string(),
        byte_low: None,
        byte_high: None,
        payload_json: Some(input.payload_json.clone()),
        payload_schema_id: Some(PAYLOAD_SCHEMA_HISTORY_GAP_V1.to_string()),
        source_event_id_hash: None,
        occurred_at_ms: input.occurred_at_ms,
        created_at_ms: input.now,
        capture_semantics: "raw_vt_stream".to_string(),
        trust_level: "system".to_string(),
        metadata_json: None,
    };
    insert_into(terminal_journal_events::table).values(&event).execute(connection)?;
    Ok(event_id)
}

pub(super) fn insert_history_gap_row(
    connection: &mut SqliteConnection,
    input: &HistoryGapTransaction<'_>,
    event_seq_low: i64,
    event_seq_high: i64,
) -> Result<(), TerminalPersistenceV2Error> {
    let gap = NewHistoryGapRow {
        id: new_id(),
        session_id: input.session_id.to_string(),
        pane_id: Some(input.pane_id.to_string()),
        stream_id: input.stream_id.to_string(),
        gap_kind: "capture_gap".to_string(),
        event_seq_low: Some(event_seq_low),
        event_seq_high: Some(event_seq_high),
        byte_low: None,
        byte_high: None,
        estimated_dropped_bytes: input.estimated_dropped_bytes,
        estimated_dropped_events: Some(input.skipped_events),
        reason: input.reason.to_string(),
        writer_generation: Some(input.writer_generation.to_string()),
        opened_at_ms: input.occurred_at_ms,
        closed_at_ms: Some(input.occurred_at_ms),
        metadata_json: None,
    };
    insert_into(terminal_history_gaps::table).values(&gap).execute(connection)?;
    Ok(())
}

pub(super) fn update_pane_last_event_seq(
    connection: &mut SqliteConnection,
    pane_id: &str,
    event_seq: i64,
) -> Result<(), TerminalPersistenceV2Error> {
    diesel::update(terminal_panes::table.filter(terminal_panes::id.eq(pane_id)))
        .set(terminal_panes::last_event_seq.eq(event_seq))
        .execute(connection)?;
    Ok(())
}
