use super::super::*;

pub(in crate::v2) fn collect_restore_replay_safety(
    connection: &mut SqliteConnection,
    session_id: &str,
) -> Result<RestoreReplaySafetyRecord, TerminalPersistenceV2Error> {
    let segments = terminal_stream_segments::table
        .filter(terminal_stream_segments::session_id.eq(session_id))
        .select(terminal_stream_segments::payload)
        .load::<Vec<u8>>(connection)?;
    let mut record = RestoreReplaySafetyRecord {
        session_id: session_id.to_string(),
        scanned_segment_count: i64::try_from(segments.len()).unwrap_or(i64::MAX),
        osc52_clipboard_count: 0,
        title_sequence_count: 0,
        hyperlink_sequence_count: 0,
        cwd_sequence_count: 0,
        shell_marker_sequence_count: 0,
        bel_byte_count: 0,
        side_effects_suppressed: true,
        prompt_injection_text_is_data: true,
    };
    for payload in segments {
        record.osc52_clipboard_count += count_byte_pattern(&payload, b"\x1b]52;");
        record.title_sequence_count +=
            count_byte_pattern(&payload, b"\x1b]0;") + count_byte_pattern(&payload, b"\x1b]2;");
        record.hyperlink_sequence_count += count_byte_pattern(&payload, b"\x1b]8;");
        record.cwd_sequence_count += count_byte_pattern(&payload, b"\x1b]7;");
        record.shell_marker_sequence_count +=
            count_byte_pattern(&payload, b"\x1b]133;") + count_byte_pattern(&payload, b"\x1b]633;");
        record.bel_byte_count += payload.iter().filter(|byte| **byte == 0x07).count() as i64;
    }
    Ok(record)
}

pub(in crate::v2) fn count_byte_pattern(haystack: &[u8], needle: &[u8]) -> i64 {
    if needle.is_empty() || haystack.len() < needle.len() {
        return 0;
    }
    haystack.windows(needle.len()).filter(|window| *window == needle).count() as i64
}
