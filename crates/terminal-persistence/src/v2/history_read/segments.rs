use super::super::*;

pub(super) struct HydratedSegments {
    pub(super) segments: Vec<StreamSegmentRecord>,
    pub(super) total_payload_bytes: i64,
    pub(super) has_more_segments: bool,
    pub(super) next_event_seq: Option<i64>,
}

pub(super) fn load_stream_segment_rows(
    connection: &mut SqliteConnection,
    session_id: &str,
    pane_id: &str,
    from_event_seq: i64,
    max_segments: i64,
) -> Result<Vec<StreamSegmentRow>, TerminalPersistenceV2Error> {
    terminal_stream_segments::table
        .filter(terminal_stream_segments::session_id.eq(session_id))
        .filter(terminal_stream_segments::pane_id.eq(pane_id))
        .filter(terminal_stream_segments::event_seq_high.ge(from_event_seq))
        .order(terminal_stream_segments::event_seq_low.asc())
        .limit(max_segments + 1)
        .select(StreamSegmentRow::as_select())
        .load::<StreamSegmentRow>(connection)
        .map_err(Into::into)
}

pub(super) fn collect_hydratable_segments(
    connection: &mut SqliteConnection,
    session_id: &str,
    rows: Vec<StreamSegmentRow>,
    max_segments: i64,
    max_bytes: i64,
    now: i64,
) -> Result<HydratedSegments, TerminalPersistenceV2Error> {
    let mut segments = Vec::new();
    let mut total_payload_bytes = 0_i64;
    let mut has_more_segments = rows.len() > max_segments as usize;

    for row in rows.into_iter().take(max_segments as usize) {
        if let Some(failure) = stream_segment_hydration_failure(&row) {
            persist_hydration_segment_failure(connection, session_id, &row, &failure, now)?;
            continue;
        }

        let row_payload_bytes = row.payload_len.max(0);
        if total_payload_bytes > 0 && total_payload_bytes + row_payload_bytes > max_bytes {
            has_more_segments = true;
            break;
        }

        total_payload_bytes += row_payload_bytes;
        segments.push(StreamSegmentRecord::from(row));
    }

    let next_event_seq = segments.last().map(|segment| segment.event_seq_high + 1);
    Ok(HydratedSegments { segments, total_payload_bytes, has_more_segments, next_event_seq })
}
