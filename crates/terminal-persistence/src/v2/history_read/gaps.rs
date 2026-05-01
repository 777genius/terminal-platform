use super::super::*;

pub(super) fn load_history_gaps(
    connection: &mut SqliteConnection,
    session_id: &str,
    pane_id: &str,
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
        .map(|rows| rows.into_iter().map(HistoryGapRecord::from).collect())
        .map_err(Into::into)
}
