use super::super::*;

pub(in crate::v2) fn load_stream_cursor(
    connection: &mut SqliteConnection,
    session_id: &str,
    pane_id: &str,
    stream_id: &str,
) -> Result<StreamCursorRow, TerminalPersistenceV2Error> {
    terminal_stream_cursors::table
        .filter(terminal_stream_cursors::session_id.eq(session_id))
        .filter(terminal_stream_cursors::pane_id.eq(pane_id))
        .filter(terminal_stream_cursors::stream_id.eq(stream_id))
        .select(StreamCursorRow::as_select())
        .first::<StreamCursorRow>(connection)
        .map_err(Into::into)
}

pub(in crate::v2) fn load_capture_receipt(
    connection: &mut SqliteConnection,
    session_id: &str,
    source_kind: &str,
    source_event_id_hash: &str,
) -> Result<Option<CaptureReceiptRow>, TerminalPersistenceV2Error> {
    terminal_capture_receipts::table
        .filter(terminal_capture_receipts::session_id.eq(session_id))
        .filter(terminal_capture_receipts::source_kind.eq(source_kind))
        .filter(terminal_capture_receipts::source_event_id_hash.eq(source_event_id_hash))
        .select(CaptureReceiptRow::as_select())
        .first::<CaptureReceiptRow>(connection)
        .optional()
        .map_err(Into::into)
}

pub(in crate::v2) fn stream_segment_receipt_from_capture_receipt(
    connection: &mut SqliteConnection,
    receipt: &CaptureReceiptRow,
) -> Result<StreamSegmentReceipt, TerminalPersistenceV2Error> {
    let commit_id = receipt.commit_id.as_deref().ok_or_else(|| {
        TerminalPersistenceV2Error::InvalidData(format!(
            "stream capture receipt {} does not point to a commit",
            receipt.id
        ))
    })?;
    stream_segment_receipt_from_commit(connection, commit_id)
}

pub(in crate::v2) fn stream_segment_receipt_from_commit(
    connection: &mut SqliteConnection,
    commit_ref: &str,
) -> Result<StreamSegmentReceipt, TerminalPersistenceV2Error> {
    let segment = terminal_stream_segments::table
        .filter(terminal_stream_segments::commit_id.eq(commit_ref))
        .select(StreamSegmentRow::as_select())
        .first::<StreamSegmentRow>(connection)?;
    let event_id = terminal_journal_events::table
        .filter(terminal_journal_events::commit_id.eq(commit_ref))
        .select(terminal_journal_events::id)
        .first::<String>(connection)?;
    let commit_seq = terminal_commit_log::table
        .filter(terminal_commit_log::id.eq(commit_ref))
        .select(terminal_commit_log::commit_seq)
        .first::<i64>(connection)?;

    Ok(StreamSegmentReceipt {
        commit_id: commit_ref.to_string(),
        commit_seq,
        segment_id: segment.id,
        event_id,
        event_seq_low: segment.event_seq_low,
        event_seq_high: segment.event_seq_high,
        byte_low: segment.byte_low,
        byte_high: segment.byte_high,
        checksum: segment.checksum,
    })
}

pub(in crate::v2) fn load_persisted_event_high_water(
    connection: &mut SqliteConnection,
    session_id: &str,
    pane_id: &str,
    stream_id: &str,
) -> Result<i64, TerminalPersistenceV2Error> {
    let segment_high = terminal_stream_segments::table
        .filter(terminal_stream_segments::session_id.eq(session_id))
        .filter(terminal_stream_segments::pane_id.eq(pane_id))
        .filter(terminal_stream_segments::stream_id.eq(stream_id))
        .select(max(terminal_stream_segments::event_seq_high))
        .first::<Option<i64>>(connection)?
        .unwrap_or(0);
    let gap_high = terminal_history_gaps::table
        .filter(terminal_history_gaps::session_id.eq(session_id))
        .filter(terminal_history_gaps::pane_id.eq(Some(pane_id.to_string())))
        .filter(terminal_history_gaps::stream_id.eq(stream_id))
        .filter(terminal_history_gaps::event_seq_high.is_not_null())
        .select(max(terminal_history_gaps::event_seq_high))
        .first::<Option<i64>>(connection)?
        .unwrap_or(0);

    Ok(segment_high.max(gap_high))
}

pub(in crate::v2) fn has_history_gap_in_range(
    connection: &mut SqliteConnection,
    session_id: &str,
    pane_id: &str,
    stream_id: &str,
    from_event_seq: i64,
    to_event_seq: i64,
) -> Result<bool, TerminalPersistenceV2Error> {
    if from_event_seq > to_event_seq {
        return Ok(false);
    }
    let count = terminal_history_gaps::table
        .filter(terminal_history_gaps::session_id.eq(session_id))
        .filter(terminal_history_gaps::pane_id.eq(Some(pane_id.to_string())))
        .filter(terminal_history_gaps::stream_id.eq(stream_id))
        .filter(terminal_history_gaps::event_seq_low.le(Some(to_event_seq)))
        .filter(terminal_history_gaps::event_seq_high.ge(Some(from_event_seq)))
        .count()
        .get_result::<i64>(connection)?;
    Ok(count > 0)
}

pub(in crate::v2) fn advance_stream_cursor(
    connection: &mut SqliteConnection,
    cursor_id: &str,
    next_event_seq: i64,
    next_byte_seq: i64,
    updated_at_ms: i64,
) -> Result<(), TerminalPersistenceV2Error> {
    diesel::update(
        terminal_stream_cursors::table.filter(terminal_stream_cursors::id.eq(cursor_id)),
    )
    .set((
        terminal_stream_cursors::next_event_seq.eq(next_event_seq),
        terminal_stream_cursors::next_byte_seq.eq(next_byte_seq),
        terminal_stream_cursors::updated_at_ms.eq(updated_at_ms),
    ))
    .execute(connection)?;
    Ok(())
}
