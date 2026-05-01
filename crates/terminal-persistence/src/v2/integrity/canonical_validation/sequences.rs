use super::super::super::*;
use super::stream_ranges::validate_stream_segment_ordering;

pub(in crate::v2) fn validate_sequence_invariants(
    connection: &mut SqliteConnection,
    session_id: Option<&str>,
    failures: &mut Vec<String>,
) -> Result<(), TerminalPersistenceV2Error> {
    validate_stream_cursor_invariants(connection, session_id, failures)?;
    validate_pane_last_event_invariants(connection, session_id, failures)?;
    validate_stream_segment_ordering(connection, session_id, failures)?;
    validate_commit_sequence_invariants(connection, session_id, failures)?;

    Ok(())
}

fn validate_stream_cursor_invariants(
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

    Ok(())
}

fn validate_pane_last_event_invariants(
    connection: &mut SqliteConnection,
    session_id: Option<&str>,
    failures: &mut Vec<String>,
) -> Result<(), TerminalPersistenceV2Error> {
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
