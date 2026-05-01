use super::super::super::super::*;
use super::context::UiInputTransaction;

pub(super) fn insert_ui_input_journal_event(
    connection: &mut SqliteConnection,
    tx: &UiInputTransaction<'_>,
    commit_id: &str,
    event_seq: i64,
) -> Result<(), TerminalPersistenceV2Error> {
    let scope = event_scope(&tx.input.session_id, Some(&tx.input.pane_id));
    let event = NewJournalEventRow {
        id: new_id(),
        session_id: tx.input.session_id.clone(),
        pane_id: Some(tx.input.pane_id.clone()),
        commit_id: commit_id.to_string(),
        stream_id: tx.stream_id.clone(),
        event_scope_kind: scope.kind,
        event_scope_id: scope.id,
        event_seq,
        event_type: tx.event_type.to_string(),
        byte_low: None,
        byte_high: None,
        payload_json: Some(tx.payload_json.clone()),
        payload_schema_id: Some(PAYLOAD_SCHEMA_UI_INPUT_V1.to_string()),
        source_event_id_hash: tx.source_event_id_hash.clone(),
        occurred_at_ms: tx.now,
        created_at_ms: tx.now,
        capture_semantics: "ui_input".to_string(),
        trust_level: "verified".to_string(),
        metadata_json: None,
    };
    insert_into(terminal_journal_events::table).values(&event).execute(connection)?;
    Ok(())
}

pub(super) fn advance_ui_input_cursor(
    connection: &mut SqliteConnection,
    cursor_id: &str,
    pane_id: &str,
    next_event_seq: i64,
    next_byte_seq: i64,
    event_seq: i64,
    now: i64,
) -> Result<(), TerminalPersistenceV2Error> {
    advance_stream_cursor(connection, cursor_id, next_event_seq, next_byte_seq, now)?;
    diesel::update(terminal_panes::table.filter(terminal_panes::id.eq(pane_id)))
        .set(terminal_panes::last_event_seq.eq(event_seq))
        .execute(connection)?;
    Ok(())
}
