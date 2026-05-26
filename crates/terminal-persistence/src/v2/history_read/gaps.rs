use super::super::*;

pub(super) fn load_history_gaps(
    connection: &mut SqliteConnection,
    session_id: &str,
    pane_id: &str,
    from_event_seq: i64,
    to_event_seq: Option<i64>,
) -> Result<Vec<HistoryGapRecord>, TerminalPersistenceV2Error> {
    terminal_history_gaps::table
        .filter(terminal_history_gaps::session_id.eq(session_id))
        .filter(
            terminal_history_gaps::pane_id.is_null().or(terminal_history_gaps::pane_id.eq(pane_id)),
        )
        .order(terminal_history_gaps::opened_at_ms.asc())
        .limit(MAX_HISTORY_GAP_LIMIT)
        .select(HistoryGapRow::as_select())
        .load::<HistoryGapRow>(connection)
        .map(|rows| {
            rows.into_iter()
                .map(HistoryGapRecord::from)
                .filter(|gap| gap_overlaps_page(gap, from_event_seq, to_event_seq))
                .collect()
        })
        .map_err(Into::into)
}

fn gap_overlaps_page(
    gap: &HistoryGapRecord,
    from_event_seq: i64,
    to_event_seq: Option<i64>,
) -> bool {
    match (gap.event_seq_low, gap.event_seq_high) {
        (Some(low), Some(high)) => {
            high >= from_event_seq && to_event_seq.map_or(true, |page_high| low <= page_high)
        }
        _ => from_event_seq <= 1,
    }
}
