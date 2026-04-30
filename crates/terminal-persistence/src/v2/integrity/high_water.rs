use super::super::*;

pub(in crate::v2) fn max_stream_segment_event_high(
    connection: &mut SqliteConnection,
    session_id: &str,
    pane_id: &str,
    stream_id: Option<&str>,
) -> Result<Option<i64>, TerminalPersistenceV2Error> {
    let mut query = terminal_stream_segments::table
        .filter(terminal_stream_segments::session_id.eq(session_id))
        .filter(terminal_stream_segments::pane_id.eq(pane_id))
        .into_boxed();
    if let Some(stream_id) = stream_id {
        query = query.filter(terminal_stream_segments::stream_id.eq(stream_id));
    }
    query
        .select(max(terminal_stream_segments::event_seq_high))
        .first::<Option<i64>>(connection)
        .map_err(Into::into)
}

pub(in crate::v2) fn max_stream_segment_byte_high(
    connection: &mut SqliteConnection,
    session_id: &str,
    pane_id: &str,
    stream_id: Option<&str>,
) -> Result<Option<i64>, TerminalPersistenceV2Error> {
    let mut query = terminal_stream_segments::table
        .filter(terminal_stream_segments::session_id.eq(session_id))
        .filter(terminal_stream_segments::pane_id.eq(pane_id))
        .into_boxed();
    if let Some(stream_id) = stream_id {
        query = query.filter(terminal_stream_segments::stream_id.eq(stream_id));
    }
    query
        .select(max(terminal_stream_segments::byte_high))
        .first::<Option<i64>>(connection)
        .map_err(Into::into)
}

pub(in crate::v2) fn max_journal_event_seq(
    connection: &mut SqliteConnection,
    session_id: &str,
    pane_id: &str,
    stream_id: Option<&str>,
) -> Result<Option<i64>, TerminalPersistenceV2Error> {
    let mut query = terminal_journal_events::table
        .filter(terminal_journal_events::session_id.eq(session_id))
        .filter(terminal_journal_events::pane_id.eq(Some(pane_id.to_string())))
        .into_boxed();
    if let Some(stream_id) = stream_id {
        query = query.filter(terminal_journal_events::stream_id.eq(stream_id));
    }
    query
        .select(max(terminal_journal_events::event_seq))
        .first::<Option<i64>>(connection)
        .map_err(Into::into)
}

pub(in crate::v2) fn max_history_gap_event_high(
    connection: &mut SqliteConnection,
    session_id: &str,
    pane_id: &str,
    stream_id: Option<&str>,
) -> Result<Option<i64>, TerminalPersistenceV2Error> {
    let mut query = terminal_history_gaps::table
        .filter(terminal_history_gaps::session_id.eq(session_id))
        .filter(terminal_history_gaps::pane_id.eq(Some(pane_id.to_string())))
        .into_boxed();
    if let Some(stream_id) = stream_id {
        query = query.filter(terminal_history_gaps::stream_id.eq(stream_id));
    }
    query
        .select(max(terminal_history_gaps::event_seq_high))
        .first::<Option<i64>>(connection)
        .map_err(Into::into)
}

pub(in crate::v2) fn max_optional_i64(values: &[Option<i64>]) -> Option<i64> {
    values.iter().filter_map(|value| *value).max()
}
