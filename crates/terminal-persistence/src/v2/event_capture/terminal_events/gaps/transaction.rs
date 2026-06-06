use super::{
    super::super::super::*,
    rows::{insert_history_gap_journal_event, insert_history_gap_row, update_pane_last_event_seq},
};

pub(super) struct HistoryGapTransaction<'a> {
    pub(super) session_id: &'a str,
    pub(super) pane_id: &'a str,
    pub(super) writer_generation: &'a str,
    pub(super) skipped_events: i64,
    pub(super) estimated_dropped_bytes: Option<i64>,
    pub(super) reason: &'a str,
    pub(super) payload_json: String,
    pub(super) occurred_at_ms: i64,
    pub(super) stream_id: &'a str,
    pub(super) now: i64,
}

pub(super) fn append_history_gap_transaction(
    connection: &mut SqliteConnection,
    input: HistoryGapTransaction<'_>,
) -> Result<JournalEventReceipt, TerminalPersistenceV2Error> {
    ensure_active_writer(connection, input.writer_generation, input.now)?;
    let commit = allocate_commit(
        connection,
        input.session_id,
        "history_gap",
        input.writer_generation,
        input.occurred_at_ms,
        input.now,
        None,
    )?;
    let cursor = load_stream_cursor(connection, input.session_id, input.pane_id, input.stream_id)?;
    let event_seq_low = cursor.next_event_seq;
    let event_seq_high = event_seq_low + input.skipped_events - 1;
    let event_id = insert_history_gap_journal_event(connection, &input, &commit.id, event_seq_low)?;
    insert_history_gap_row(connection, &input, event_seq_low, event_seq_high)?;
    advance_stream_cursor(
        connection,
        &cursor.id,
        event_seq_high + 1,
        cursor.next_byte_seq,
        input.now,
    )?;
    update_pane_last_event_seq(connection, input.pane_id, event_seq_high)?;

    Ok(JournalEventReceipt {
        commit_id: commit.id,
        commit_seq: commit.commit_seq,
        event_id,
        event_seq: event_seq_low,
    })
}
