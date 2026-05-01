use super::super::super::*;
use super::registration::{finish_writer_operation, upsert_history_gap_target};

impl TerminalPersistenceV2 {
    pub fn record_history_gap_event(
        &self,
        input: HistoryGapEventInput,
    ) -> Result<JournalEventReceipt, TerminalPersistenceV2Error> {
        upsert_history_gap_target(self, &input)?;

        let lease = self.acquire_writer_generation_with_retry("runtime-output-gap", 60_000)?;
        let append_result = self.append_history_gap_event(
            &input.session_id,
            &input.pane_id,
            &lease.id,
            input.skipped_events,
            input.estimated_dropped_bytes,
            &input.reason,
            input.occurred_at_ms,
        );
        let release_result = self.release_writer_generation(&lease.id);
        finish_writer_operation(append_result, release_result)
    }

    pub(in crate::v2) fn append_history_gap_event(
        &self,
        session_id: &str,
        pane_id: &str,
        writer_generation: &str,
        skipped_events: u64,
        estimated_dropped_bytes: Option<i64>,
        reason: &str,
        occurred_at_ms: Option<i64>,
    ) -> Result<JournalEventReceipt, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        let occurred_at_ms = occurred_at_ms.unwrap_or(now);
        let stream_id = DEFAULT_STREAM_ID.to_string();
        let gap_width = u64_to_i64(skipped_events.max(1), "history gap skipped events")?;
        let estimated_dropped_bytes = estimated_dropped_bytes.map(|value| value.max(0));
        let payload_json = history_gap_payload(reason, skipped_events, estimated_dropped_bytes)?;

        connection.transaction::<_, TerminalPersistenceV2Error, _>(|connection| {
            append_history_gap_transaction(
                connection,
                HistoryGapTransaction {
                    session_id,
                    pane_id,
                    writer_generation,
                    skipped_events: gap_width,
                    estimated_dropped_bytes,
                    reason,
                    payload_json,
                    occurred_at_ms,
                    stream_id: &stream_id,
                    now,
                },
            )
        })
    }
}

struct HistoryGapTransaction<'a> {
    session_id: &'a str,
    pane_id: &'a str,
    writer_generation: &'a str,
    skipped_events: i64,
    estimated_dropped_bytes: Option<i64>,
    reason: &'a str,
    payload_json: String,
    occurred_at_ms: i64,
    stream_id: &'a str,
    now: i64,
}

fn history_gap_payload(
    reason: &str,
    skipped_events: u64,
    estimated_dropped_bytes: Option<i64>,
) -> Result<String, TerminalPersistenceV2Error> {
    Ok(serde_json::to_string(&serde_json::json!({
        "reason": reason,
        "skipped_events": skipped_events,
        "estimated_dropped_bytes": estimated_dropped_bytes
    }))?)
}

fn append_history_gap_transaction(
    connection: &mut SqliteConnection,
    input: HistoryGapTransaction<'_>,
) -> Result<JournalEventReceipt, TerminalPersistenceV2Error> {
    ensure_active_writer(connection, input.writer_generation, input.now)?;
    let commit = allocate_commit(
        connection,
        input.session_id,
        "history_gap",
        input.writer_generation,
        input.occurred_at_ms,
        input.now,
        None,
    )?;
    let cursor = load_stream_cursor(connection, input.session_id, input.pane_id, input.stream_id)?;
    let event_seq_low = cursor.next_event_seq;
    let event_seq_high = event_seq_low + input.skipped_events - 1;
    let event_id = insert_history_gap_journal_event(connection, &input, &commit.id, event_seq_low)?;
    insert_history_gap_row(connection, &input, event_seq_low, event_seq_high)?;
    advance_stream_cursor(
        connection,
        &cursor.id,
        event_seq_high + 1,
        cursor.next_byte_seq,
        input.now,
    )?;
    update_pane_last_event_seq(connection, input.pane_id, event_seq_high)?;

    Ok(JournalEventReceipt {
        commit_id: commit.id,
        commit_seq: commit.commit_seq,
        event_id,
        event_seq: event_seq_low,
    })
}

fn insert_history_gap_journal_event(
    connection: &mut SqliteConnection,
    input: &HistoryGapTransaction<'_>,
    commit_id: &str,
    event_seq: i64,
) -> Result<String, TerminalPersistenceV2Error> {
    let scope = event_scope(input.session_id, Some(input.pane_id));
    let event_id = new_id();
    let event = NewJournalEventRow {
        id: event_id.clone(),
        session_id: input.session_id.to_string(),
        pane_id: Some(input.pane_id.to_string()),
        commit_id: commit_id.to_string(),
        stream_id: input.stream_id.to_string(),
        event_scope_kind: scope.kind,
        event_scope_id: scope.id,
        event_seq,
        event_type: "history_gap".to_string(),
        byte_low: None,
        byte_high: None,
        payload_json: Some(input.payload_json.clone()),
        payload_schema_id: Some(PAYLOAD_SCHEMA_HISTORY_GAP_V1.to_string()),
        source_event_id_hash: None,
        occurred_at_ms: input.occurred_at_ms,
        created_at_ms: input.now,
        capture_semantics: "raw_vt_stream".to_string(),
        trust_level: "system".to_string(),
        metadata_json: None,
    };
    insert_into(terminal_journal_events::table).values(&event).execute(connection)?;
    Ok(event_id)
}

fn insert_history_gap_row(
    connection: &mut SqliteConnection,
    input: &HistoryGapTransaction<'_>,
    event_seq_low: i64,
    event_seq_high: i64,
) -> Result<(), TerminalPersistenceV2Error> {
    let gap = NewHistoryGapRow {
        id: new_id(),
        session_id: input.session_id.to_string(),
        pane_id: Some(input.pane_id.to_string()),
        stream_id: input.stream_id.to_string(),
        gap_kind: "capture_gap".to_string(),
        event_seq_low: Some(event_seq_low),
        event_seq_high: Some(event_seq_high),
        byte_low: None,
        byte_high: None,
        estimated_dropped_bytes: input.estimated_dropped_bytes,
        estimated_dropped_events: Some(input.skipped_events),
        reason: input.reason.to_string(),
        writer_generation: Some(input.writer_generation.to_string()),
        opened_at_ms: input.occurred_at_ms,
        closed_at_ms: Some(input.occurred_at_ms),
        metadata_json: None,
    };
    insert_into(terminal_history_gaps::table).values(&gap).execute(connection)?;
    Ok(())
}

fn update_pane_last_event_seq(
    connection: &mut SqliteConnection,
    pane_id: &str,
    event_seq: i64,
) -> Result<(), TerminalPersistenceV2Error> {
    diesel::update(terminal_panes::table.filter(terminal_panes::id.eq(pane_id)))
        .set(terminal_panes::last_event_seq.eq(event_seq))
        .execute(connection)?;
    Ok(())
}
