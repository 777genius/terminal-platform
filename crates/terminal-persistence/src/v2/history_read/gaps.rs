use super::super::*;

pub(super) fn load_history_gaps(
    connection: &mut SqliteConnection,
    session_id: &str,
    pane_id: &str,
    from_event_seq: i64,
    to_event_seq: Option<i64>,
) -> Result<Vec<HistoryGapRecord>, TerminalPersistenceV2Error> {
    let mut rows = terminal_history_gaps::table
        .filter(terminal_history_gaps::session_id.eq(session_id))
        .filter(
            terminal_history_gaps::pane_id.is_null().or(terminal_history_gaps::pane_id.eq(pane_id)),
        )
        .filter(terminal_history_gaps::event_seq_low.is_not_null())
        .filter(terminal_history_gaps::event_seq_high.is_not_null())
        .filter(terminal_history_gaps::event_seq_high.ge(Some(from_event_seq)))
        .into_boxed();

    if let Some(to_event_seq) = to_event_seq {
        rows = rows.filter(terminal_history_gaps::event_seq_low.le(Some(to_event_seq)));
    }

    let mut gaps = rows
        .order((
            terminal_history_gaps::event_seq_low.asc(),
            terminal_history_gaps::opened_at_ms.asc(),
        ))
        .limit(MAX_HISTORY_GAP_LIMIT)
        .select(HistoryGapRow::as_select())
        .load::<HistoryGapRow>(connection)
        .map_err(TerminalPersistenceV2Error::from)?
        .into_iter()
        .map(HistoryGapRecord::from)
        .collect::<Vec<_>>();

    if from_event_seq <= 1 && gaps.len() < MAX_HISTORY_GAP_LIMIT as usize {
        let remaining_limit = MAX_HISTORY_GAP_LIMIT - gaps.len() as i64;
        let unknown_rows = terminal_history_gaps::table
            .filter(terminal_history_gaps::session_id.eq(session_id))
            .filter(
                terminal_history_gaps::pane_id
                    .is_null()
                    .or(terminal_history_gaps::pane_id.eq(pane_id)),
            )
            .filter(
                terminal_history_gaps::event_seq_low
                    .is_null()
                    .and(terminal_history_gaps::event_seq_high.is_null()),
            )
            .order(terminal_history_gaps::opened_at_ms.asc())
            .limit(remaining_limit)
            .select(HistoryGapRow::as_select())
            .load::<HistoryGapRow>(connection)
            .map_err(TerminalPersistenceV2Error::from)?;

        gaps.extend(unknown_rows.into_iter().map(HistoryGapRecord::from));
        gaps.sort_by_key(|gap| gap.opened_at_ms);
        gaps.truncate(MAX_HISTORY_GAP_LIMIT as usize);
    }

    Ok(gaps)
}

pub(super) fn next_event_seq_after_gaps(gaps: &[HistoryGapRecord]) -> Option<i64> {
    gaps.iter().filter_map(|gap| gap.event_seq_high?.checked_add(1)).max()
}

pub(super) fn has_history_gap_at_or_after(
    connection: &mut SqliteConnection,
    session_id: &str,
    pane_id: &str,
    from_event_seq: i64,
) -> Result<bool, TerminalPersistenceV2Error> {
    let known_count = terminal_history_gaps::table
        .filter(terminal_history_gaps::session_id.eq(session_id))
        .filter(
            terminal_history_gaps::pane_id.is_null().or(terminal_history_gaps::pane_id.eq(pane_id)),
        )
        .filter(terminal_history_gaps::event_seq_low.is_not_null())
        .filter(terminal_history_gaps::event_seq_high.is_not_null())
        .filter(terminal_history_gaps::event_seq_high.ge(Some(from_event_seq)))
        .count()
        .get_result::<i64>(connection)?;
    if known_count > 0 {
        return Ok(true);
    }

    if from_event_seq > 1 {
        return Ok(false);
    }

    let unknown_count = terminal_history_gaps::table
        .filter(terminal_history_gaps::session_id.eq(session_id))
        .filter(
            terminal_history_gaps::pane_id.is_null().or(terminal_history_gaps::pane_id.eq(pane_id)),
        )
        .filter(
            terminal_history_gaps::event_seq_low
                .is_null()
                .and(terminal_history_gaps::event_seq_high.is_null()),
        )
        .count()
        .get_result::<i64>(connection)?;
    Ok(unknown_count > 0)
}
