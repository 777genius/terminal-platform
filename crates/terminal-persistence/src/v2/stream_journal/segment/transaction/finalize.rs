use super::super::super::super::*;

pub(super) fn final_event_seq(
    event_seq_high: i64,
    transition_count: usize,
) -> Result<i64, TerminalPersistenceV2Error> {
    let transition_count = checked_len(transition_count, "buffer mode transition count")?;
    event_seq_high.checked_add(transition_count).ok_or_else(|| {
        TerminalPersistenceV2Error::InvalidData(
            "buffer mode transition event sequence overflow".to_string(),
        )
    })
}

pub(super) fn finalize_stream_segment(
    connection: &mut SqliteConnection,
    cursor_id: &str,
    pane_id: &str,
    final_event_seq: i64,
    byte_high: i64,
    now: i64,
) -> Result<(), TerminalPersistenceV2Error> {
    advance_stream_cursor(connection, cursor_id, final_event_seq + 1, byte_high, now)?;
    diesel::update(terminal_panes::table.filter(terminal_panes::id.eq(pane_id)))
        .set(terminal_panes::last_event_seq.eq(final_event_seq))
        .execute(connection)?;
    Ok(())
}
