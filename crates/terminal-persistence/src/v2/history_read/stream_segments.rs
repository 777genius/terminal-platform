use super::super::*;

impl TerminalPersistenceV2 {
    pub fn list_stream_segments(
        &self,
        session_id: &str,
        pane_id: &str,
        from_event_seq: i64,
        limit: i64,
    ) -> Result<Vec<StreamSegmentRecord>, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        terminal_stream_segments::table
            .filter(terminal_stream_segments::session_id.eq(session_id))
            .filter(terminal_stream_segments::pane_id.eq(pane_id))
            .filter(terminal_stream_segments::event_seq_high.ge(from_event_seq))
            .order(terminal_stream_segments::event_seq_low.asc())
            .limit(limit)
            .select(StreamSegmentRow::as_select())
            .load::<StreamSegmentRow>(&mut connection)
            .map(|rows| rows.into_iter().map(StreamSegmentRecord::from).collect())
            .map_err(Into::into)
    }
}
