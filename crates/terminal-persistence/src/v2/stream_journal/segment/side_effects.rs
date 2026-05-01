use super::super::super::*;

pub(super) struct ProjectionOutboxInput<'a> {
    pub(super) session_id: &'a str,
    pub(super) pane_id: &'a str,
    pub(super) stream_id: &'a str,
    pub(super) commit_id: &'a str,
    pub(super) event_seq_low: i64,
    pub(super) event_seq_high: i64,
    pub(super) byte_low: i64,
    pub(super) byte_high: i64,
    pub(super) now: i64,
}

pub(super) struct CaptureReceiptInput<'a> {
    pub(super) session_id: &'a str,
    pub(super) commit_id: &'a str,
    pub(super) source_kind: &'a str,
    pub(super) source_event_id_hash: &'a str,
    pub(super) source_payload_hash: &'a str,
    pub(super) received_at_ms: i64,
    pub(super) created_at_ms: i64,
    pub(super) metadata_json: Option<String>,
}

pub(super) fn enqueue_pane_history_projection(
    connection: &mut SqliteConnection,
    input: ProjectionOutboxInput<'_>,
) -> Result<(), TerminalPersistenceV2Error> {
    let outbox = NewOutboxMessageRow {
        id: new_id(),
        message_kind: "pane_history_projection".to_string(),
        dedupe_key: Some(normalize_outbox_dedupe_key(&format!(
            "pane_history_projection:{}",
            input.commit_id
        ))),
        state: "pending".to_string(),
        payload_json: serde_json::to_string(&serde_json::json!({
            "session_id": input.session_id,
            "pane_id": input.pane_id,
            "stream_id": input.stream_id,
            "commit_id": input.commit_id,
            "event_seq_low": input.event_seq_low,
            "event_seq_high": input.event_seq_high,
            "byte_low": input.byte_low,
            "byte_high": input.byte_high
        }))?,
        attempts: 0,
        max_attempts: 5,
        claimed_by: None,
        lease_token: None,
        claimed_until_ms: None,
        next_run_at_ms: input.now,
        last_error: None,
        created_at_ms: input.now,
        updated_at_ms: input.now,
    };
    insert_into(terminal_outbox_messages::table).values(&outbox).execute(connection)?;
    Ok(())
}

pub(super) fn insert_capture_receipt(
    connection: &mut SqliteConnection,
    input: CaptureReceiptInput<'_>,
) -> Result<(), TerminalPersistenceV2Error> {
    let receipt = NewCaptureReceiptRow {
        id: new_id(),
        session_id: input.session_id.to_string(),
        commit_id: Some(input.commit_id.to_string()),
        source_kind: input.source_kind.to_string(),
        source_event_id_hash: input.source_event_id_hash.to_string(),
        source_payload_hash: input.source_payload_hash.to_string(),
        received_at_ms: input.received_at_ms,
        created_at_ms: input.created_at_ms,
        metadata_json: input.metadata_json,
    };
    insert_into(terminal_capture_receipts::table).values(&receipt).execute(connection)?;
    Ok(())
}
