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
    pane_id: &str,
    rows: Vec<StreamSegmentRow>,
    max_segments: i64,
    max_bytes: i64,
    now: i64,
) -> Result<HydratedSegments, TerminalPersistenceV2Error> {
    let mut segments = Vec::new();
    let mut total_payload_bytes = 0_i64;
    let mut has_more_segments = false;
    let mut next_event_seq = None;

    for row in rows {
        if segments.len() >= max_segments as usize {
            has_more_segments = true;
            break;
        }

        let row_next_event_seq = valid_next_event_seq(&row);
        if let Some(failure) = stream_segment_hydration_failure(&row) {
            persist_hydration_segment_failure(connection, session_id, &row, &failure, now)?;
            next_event_seq = row_next_event_seq.or(next_event_seq);
            continue;
        }

        let row_payload_bytes = row.payload_len.max(0);
        if total_payload_bytes > 0 && total_payload_bytes + row_payload_bytes > max_bytes {
            has_more_segments = true;
            break;
        }

        total_payload_bytes += row_payload_bytes;
        next_event_seq = row_next_event_seq;
        segments.push(StreamSegmentRecord::from(row));
    }

    if !has_more_segments {
        has_more_segments = next_event_seq
            .map(|event_seq| {
                has_more_stream_segments_at_or_after(connection, session_id, pane_id, event_seq)
            })
            .transpose()?
            .unwrap_or(false);
    }

    Ok(HydratedSegments { segments, total_payload_bytes, has_more_segments, next_event_seq })
}

fn valid_next_event_seq(row: &StreamSegmentRow) -> Option<i64> {
    if row.event_seq_low > row.event_seq_high {
        return None;
    }

    row.event_seq_high.checked_add(1)
}

fn has_more_stream_segments_at_or_after(
    connection: &mut SqliteConnection,
    session_id: &str,
    pane_id: &str,
    from_event_seq: i64,
) -> Result<bool, TerminalPersistenceV2Error> {
    let count = terminal_stream_segments::table
        .filter(terminal_stream_segments::session_id.eq(session_id))
        .filter(terminal_stream_segments::pane_id.eq(pane_id))
        .filter(terminal_stream_segments::event_seq_high.ge(from_event_seq))
        .count()
        .get_result::<i64>(connection)?;
    Ok(count > 0)
}
