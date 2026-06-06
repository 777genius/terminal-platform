use super::super::super::*;

pub(super) struct BufferModeTransitionInput<'a> {
    pub(super) transitions: &'a [BufferModeTransition],
    pub(super) session_id: &'a str,
    pub(super) pane_id: &'a str,
    pub(super) commit_id: &'a str,
    pub(super) stream_id: &'a str,
    pub(super) segment_id: &'a str,
    pub(super) event_seq_low: i64,
    pub(super) byte_low: i64,
    pub(super) byte_high: i64,
    pub(super) occurred_at_ms: i64,
    pub(super) now: i64,
    pub(super) capture_semantics: &'a str,
}

pub(super) fn insert_buffer_mode_transition_events(
    connection: &mut SqliteConnection,
    input: BufferModeTransitionInput<'_>,
) -> Result<(), TerminalPersistenceV2Error> {
    for (transition_index, transition) in input.transitions.iter().enumerate() {
        let transition_offset = checked_len(transition_index + 1, "buffer mode transition offset")?;
        let transition_event_seq =
            input.event_seq_low.checked_add(transition_offset).ok_or_else(|| {
                TerminalPersistenceV2Error::InvalidData(
                    "buffer mode transition event sequence overflow".to_string(),
                )
            })?;
        let transition_byte_low =
            input.byte_low.checked_add(transition.byte_offset).ok_or_else(|| {
                TerminalPersistenceV2Error::InvalidData(
                    "buffer mode transition byte range overflow".to_string(),
                )
            })?;
        let transition_byte_high =
            transition_byte_low.checked_add(transition.byte_len).ok_or_else(|| {
                TerminalPersistenceV2Error::InvalidData(
                    "buffer mode transition byte range overflow".to_string(),
                )
            })?;
        let payload_json = serde_json::to_string(&serde_json::json!({
            "action": transition.action,
            "mode": transition.mode,
            "target_buffer_kind": transition.target_buffer_kind,
            "derived_from_event_seq": input.event_seq_low
        }))?;
        let transition_event = NewJournalEventRow {
            id: new_id(),
            session_id: input.session_id.to_string(),
            pane_id: Some(input.pane_id.to_string()),
            commit_id: input.commit_id.to_string(),
            stream_id: input.stream_id.to_string(),
            event_scope_kind: "pane".to_string(),
            event_scope_id: input.pane_id.to_string(),
            event_seq: transition_event_seq,
            event_type: "terminal_buffer_mode".to_string(),
            byte_low: Some(transition_byte_low),
            byte_high: Some(transition_byte_high.min(input.byte_high)),
            payload_json: Some(payload_json),
            payload_schema_id: Some(PAYLOAD_SCHEMA_JOURNAL_EVENT_V1.to_string()),
            source_event_id_hash: None,
            occurred_at_ms: input.occurred_at_ms,
            created_at_ms: input.now,
            capture_semantics: input.capture_semantics.to_string(),
            trust_level: "parser_derived".to_string(),
            metadata_json: Some(serde_json::to_string(&serde_json::json!({
                "parser": "terminal_buffer_mode_detector_v1",
                "source_segment_id": input.segment_id
            }))?),
        };
        insert_into(terminal_journal_events::table)
            .values(&transition_event)
            .execute(connection)?;
    }

    Ok(())
}
