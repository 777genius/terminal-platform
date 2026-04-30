use super::super::*;
use super::*;

pub(in crate::v2) fn validate_checksum_bytes(
    row_kind: &str,
    id: &str,
    payload: &[u8],
    algorithm: &str,
    expected: &str,
    failures: &mut Vec<String>,
) {
    if algorithm != "blake3" {
        failures.push(format!("{row_kind}:{id} uses unsupported checksum algorithm {algorithm}"));
        return;
    }
    let actual = blake3_hash_bytes(payload);
    if actual != expected {
        failures.push(format!("{row_kind}:{id} checksum mismatch"));
    }
}

pub(in crate::v2) fn validate_checksum_text(
    row_kind: &str,
    id: &str,
    payload: &str,
    algorithm: &str,
    expected: &str,
    failures: &mut Vec<String>,
) {
    validate_checksum_bytes(row_kind, id, payload.as_bytes(), algorithm, expected, failures);
}

pub(in crate::v2) fn validate_payload_schema_ref(
    row_kind: &str,
    id: &str,
    payload_present: bool,
    payload_schema_id: Option<&str>,
    schema_ids: &[String],
    failures: &mut Vec<String>,
) {
    if !payload_present {
        return;
    }
    let Some(payload_schema_id) = payload_schema_id else {
        failures.push(format!("{row_kind}:{id} missing payload_schema_id"));
        return;
    };
    if !schema_ids.iter().any(|schema_id| schema_id == payload_schema_id) {
        failures.push(format!(
            "{row_kind}:{id} references unknown payload_schema_id {payload_schema_id}"
        ));
    }
}

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

pub(in crate::v2) fn validate_sequence_invariants(
    connection: &mut SqliteConnection,
    session_id: Option<&str>,
    failures: &mut Vec<String>,
) -> Result<(), TerminalPersistenceV2Error> {
    let mut cursor_query = terminal_stream_cursors::table.into_boxed();
    if let Some(session_id) = session_id {
        cursor_query = cursor_query.filter(terminal_stream_cursors::session_id.eq(session_id));
    }
    let cursors =
        cursor_query.select(StreamCursorRow::as_select()).load::<StreamCursorRow>(connection)?;
    for cursor in cursors {
        let segment_event_high = max_stream_segment_event_high(
            connection,
            &cursor.session_id,
            &cursor.pane_id,
            Some(&cursor.stream_id),
        )?;
        let journal_event_high = max_journal_event_seq(
            connection,
            &cursor.session_id,
            &cursor.pane_id,
            Some(&cursor.stream_id),
        )?;
        let gap_event_high = max_history_gap_event_high(
            connection,
            &cursor.session_id,
            &cursor.pane_id,
            Some(&cursor.stream_id),
        )?;
        if let Some(observed_event_high) =
            max_optional_i64(&[segment_event_high, journal_event_high, gap_event_high])
        {
            let expected_next_event_seq = observed_event_high + 1;
            if cursor.next_event_seq != expected_next_event_seq {
                failures.push(format!(
                    "stream_cursor:{} next_event_seq={} expected={}",
                    cursor.id, cursor.next_event_seq, expected_next_event_seq
                ));
            }
        }

        let expected_next_byte_seq = max_stream_segment_byte_high(
            connection,
            &cursor.session_id,
            &cursor.pane_id,
            Some(&cursor.stream_id),
        )?
        .unwrap_or(0);
        if cursor.next_byte_seq != expected_next_byte_seq {
            failures.push(format!(
                "stream_cursor:{} next_byte_seq={} expected={}",
                cursor.id, cursor.next_byte_seq, expected_next_byte_seq
            ));
        }
    }

    let mut pane_query = terminal_panes::table.into_boxed();
    if let Some(session_id) = session_id {
        pane_query = pane_query.filter(terminal_panes::session_id.eq(session_id));
    }
    let panes = pane_query
        .select((terminal_panes::id, terminal_panes::session_id, terminal_panes::last_event_seq))
        .load::<(String, String, i64)>(connection)?;
    for (pane_id, pane_session_id, last_event_seq) in panes {
        let segment_event_high =
            max_stream_segment_event_high(connection, &pane_session_id, &pane_id, None)?;
        let journal_event_high =
            max_journal_event_seq(connection, &pane_session_id, &pane_id, None)?;
        let gap_event_high =
            max_history_gap_event_high(connection, &pane_session_id, &pane_id, None)?;
        if let Some(expected_last_event_seq) =
            max_optional_i64(&[segment_event_high, journal_event_high, gap_event_high])
        {
            if last_event_seq != expected_last_event_seq {
                failures.push(format!(
                    "pane:{} last_event_seq={} expected={}",
                    pane_id, last_event_seq, expected_last_event_seq
                ));
            }
        }
    }

    validate_stream_segment_ordering(connection, session_id, failures)?;
    validate_commit_sequence_invariants(connection, session_id, failures)?;

    Ok(())
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

pub(in crate::v2) fn validate_commit_sequence_invariants(
    connection: &mut SqliteConnection,
    session_id: Option<&str>,
    failures: &mut Vec<String>,
) -> Result<(), TerminalPersistenceV2Error> {
    let mut cursor_query = terminal_session_cursors::table.into_boxed();
    if let Some(session_id) = session_id {
        cursor_query = cursor_query.filter(terminal_session_cursors::session_id.eq(session_id));
    }
    let cursors = cursor_query
        .select((terminal_session_cursors::session_id, terminal_session_cursors::next_commit_seq))
        .load::<(String, i64)>(connection)?;
    for (cursor_session_id, next_commit_seq) in cursors {
        let max_commit_seq = terminal_commit_log::table
            .filter(terminal_commit_log::session_id.eq(&cursor_session_id))
            .select(max(terminal_commit_log::commit_seq))
            .first::<Option<i64>>(connection)?
            .unwrap_or(0);
        let expected_next_commit_seq = max_commit_seq + 1;
        if next_commit_seq != expected_next_commit_seq {
            failures.push(format!(
                "session_cursor:{cursor_session_id} next_commit_seq={next_commit_seq} expected={expected_next_commit_seq}"
            ));
        }
    }

    let mut commit_query = terminal_commit_log::table.into_boxed();
    if let Some(session_id) = session_id {
        commit_query = commit_query.filter(terminal_commit_log::session_id.eq(session_id));
    }
    let commits = commit_query
        .order((terminal_commit_log::session_id.asc(), terminal_commit_log::commit_seq.asc()))
        .select((
            terminal_commit_log::id,
            terminal_commit_log::session_id,
            terminal_commit_log::commit_seq,
        ))
        .load::<(String, String, i64)>(connection)?;

    let mut previous_session: Option<String> = None;
    let mut expected_commit_seq = 1_i64;
    for (commit_id, commit_session_id, commit_seq) in commits {
        if previous_session.as_deref() != Some(commit_session_id.as_str()) {
            previous_session = Some(commit_session_id.clone());
            expected_commit_seq = 1;
        }
        if commit_seq != expected_commit_seq {
            failures.push(format!(
                "commit_log:{commit_id} commit_seq={commit_seq} expected={expected_commit_seq}"
            ));
            expected_commit_seq = commit_seq + 1;
        } else {
            expected_commit_seq += 1;
        }
    }

    Ok(())
}
