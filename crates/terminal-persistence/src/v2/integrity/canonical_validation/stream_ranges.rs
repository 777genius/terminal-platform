use super::super::super::*;

pub(in crate::v2) fn validate_stream_segment_ranges(
    row: &StreamSegmentRow,
    failures: &mut Vec<String>,
) {
    if row.event_seq_high < row.event_seq_low {
        failures.push(format!(
            "stream_segment:{} invalid event range {}..{}",
            row.id, row.event_seq_low, row.event_seq_high
        ));
    }
    if row.byte_high < row.byte_low {
        failures.push(format!(
            "stream_segment:{} invalid byte range {}..{}",
            row.id, row.byte_low, row.byte_high
        ));
        return;
    }
    let expected_payload_len = row.byte_high - row.byte_low;
    if row.payload_len != expected_payload_len {
        failures.push(format!(
            "stream_segment:{} payload_len={} expected={}",
            row.id, row.payload_len, expected_payload_len
        ));
    }
    if row.stored_byte_len != i64::try_from(row.payload.len()).unwrap_or(i64::MAX) {
        failures.push(format!(
            "stream_segment:{} stored_byte_len={} actual={}",
            row.id,
            row.stored_byte_len,
            row.payload.len()
        ));
    }
}

pub(in crate::v2) fn validate_stream_segment_ordering(
    connection: &mut SqliteConnection,
    session_id: Option<&str>,
    failures: &mut Vec<String>,
) -> Result<(), TerminalPersistenceV2Error> {
    let mut query = terminal_stream_segments::table.into_boxed();
    if let Some(session_id) = session_id {
        query = query.filter(terminal_stream_segments::session_id.eq(session_id));
    }
    let rows = query
        .order((
            terminal_stream_segments::session_id.asc(),
            terminal_stream_segments::pane_id.asc(),
            terminal_stream_segments::stream_id.asc(),
            terminal_stream_segments::event_seq_low.asc(),
            terminal_stream_segments::byte_low.asc(),
        ))
        .select((
            terminal_stream_segments::id,
            terminal_stream_segments::session_id,
            terminal_stream_segments::pane_id,
            terminal_stream_segments::stream_id,
            terminal_stream_segments::event_seq_low,
            terminal_stream_segments::event_seq_high,
            terminal_stream_segments::byte_low,
            terminal_stream_segments::byte_high,
        ))
        .load::<(String, String, String, String, i64, i64, i64, i64)>(connection)?;

    let mut previous: Option<(String, String, String, String, i64, i64)> = None;
    for (
        id,
        row_session_id,
        pane_id,
        stream_id,
        event_seq_low,
        event_seq_high,
        byte_low,
        byte_high,
    ) in rows
    {
        if let Some((
            previous_id,
            previous_session_id,
            previous_pane_id,
            previous_stream_id,
            previous_event_high,
            previous_byte_high,
        )) = previous.as_ref()
        {
            if previous_session_id == &row_session_id
                && previous_pane_id == &pane_id
                && previous_stream_id == &stream_id
            {
                if event_seq_low <= *previous_event_high {
                    failures.push(format!(
                        "stream_segment:{id} overlaps stream_segment:{previous_id} event range"
                    ));
                }
                if byte_low < *previous_byte_high {
                    failures.push(format!(
                        "stream_segment:{id} overlaps stream_segment:{previous_id} byte range"
                    ));
                }
            }
        }
        previous = Some((id, row_session_id, pane_id, stream_id, event_seq_high, byte_high));
    }

    Ok(())
}
