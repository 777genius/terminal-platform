use super::super::*;

impl TerminalPersistenceV2 {
    pub fn append_stream_segment(
        &self,
        input: StreamSegmentInput,
    ) -> Result<StreamSegmentReceipt, TerminalPersistenceV2Error> {
        if input.payload.is_empty() {
            return Err(TerminalPersistenceV2Error::InvalidData(
                "stream segment payload must not be empty".to_string(),
            ));
        }
        if input.event_count != 1 {
            return Err(TerminalPersistenceV2Error::InvalidData(
                "stream segment MVP accepts exactly one journal event per segment".to_string(),
            ));
        }
        if self.config.failpoints.stream_segment_before_transaction_storage_full {
            self.record_storage_pressure_write_failure(
                "append_stream_segment",
                "synthetic_sqlite_full",
                None,
            )?;
            return Err(TerminalPersistenceV2Error::InvalidData(
                "failpoint stream_segment_before_transaction_storage_full".to_string(),
            ));
        }

        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        let occurred_at_ms = input.occurred_at_ms.unwrap_or(now);
        let stream_id = input.stream_id.unwrap_or_else(|| DEFAULT_STREAM_ID.to_string());
        let payload_len = checked_len(input.payload.len(), "payload length")?;
        let payload_checksum = blake3_hash_bytes(&input.payload);
        let metadata_json = json_metadata(&input.metadata)?;
        let source_event_id_hash = input.source_event_id_hash.clone();
        let capture_source_kind = source_event_id_hash
            .as_ref()
            .map(|_| stream_capture_source_kind(&input.pane_id, &stream_id));
        let buffer_mode_transitions = detect_buffer_mode_transitions(&input.payload);

        let append_result =
            connection.transaction::<_, TerminalPersistenceV2Error, _>(|connection| {
                if let (Some(source_kind), Some(source_event_id_hash)) =
                    (capture_source_kind.as_deref(), source_event_id_hash.as_deref())
                {
                    if let Some(receipt) = load_capture_receipt(
                        connection,
                        &input.session_id,
                        source_kind,
                        source_event_id_hash,
                    )? {
                        if receipt.source_payload_hash != payload_checksum {
                            return Err(TerminalPersistenceV2Error::InvalidData(format!(
                                "capture receipt payload hash mismatch for source_kind={source_kind}"
                            )));
                        }
                        return stream_segment_receipt_from_capture_receipt(connection, &receipt);
                    }
                }

                ensure_active_writer(connection, &input.writer_generation, now)?;
                let commit = allocate_commit(
                    connection,
                    &input.session_id,
                    "stream_segment",
                    &input.writer_generation,
                    occurred_at_ms,
                    now,
                    None,
                )?;
                let cursor =
                    load_stream_cursor(connection, &input.session_id, &input.pane_id, &stream_id)?;
                let event_seq_low = cursor.next_event_seq;
                let event_seq_high = cursor.next_event_seq;
                let byte_low = cursor.next_byte_seq;
                let byte_high = cursor.next_byte_seq + payload_len;
                let segment_id = new_id();
                let event_id = new_id();
                let capture_semantics =
                    input.capture_semantics.unwrap_or_else(|| "raw_vt_stream".to_string());
                validate_capture_semantics_domain(&capture_semantics)?;
                let event_type = input.event_type.unwrap_or_else(|| "terminal_output".to_string());
                let payload_json =
                    input.payload_json.as_ref().map(serde_json::to_string).transpose()?;
                let payload_schema_id = payload_json
                    .as_ref()
                    .map(|_| payload_schema_id_for_journal_event(&event_type).to_string());
                let transition_count =
                    checked_len(buffer_mode_transitions.len(), "buffer mode transition count")?;
                let final_event_seq = event_seq_high.checked_add(transition_count).ok_or_else(|| {
                    TerminalPersistenceV2Error::InvalidData(
                        "buffer mode transition event sequence overflow".to_string(),
                    )
                })?;

                let segment = NewStreamSegmentRow {
                    id: segment_id.clone(),
                    session_id: input.session_id.clone(),
                    pane_id: input.pane_id.clone(),
                    commit_id: commit.id.clone(),
                    stream_id: stream_id.clone(),
                    event_seq_low,
                    event_seq_high,
                    byte_low,
                    byte_high,
                    payload: input.payload.clone(),
                    payload_len,
                    stored_byte_len: payload_len,
                    uncompressed_byte_len: Some(payload_len),
                    checksum_algorithm: "blake3".to_string(),
                    checksum: payload_checksum.clone(),
                    compression: "none".to_string(),
                    capture_semantics: capture_semantics.clone(),
                    encryption_state: "plaintext".to_string(),
                    key_ref: None,
                    created_at_ms: now,
                    writer_generation: input.writer_generation.clone(),
                    metadata_json: metadata_json.clone(),
                };
                insert_into(terminal_stream_segments::table).values(&segment).execute(connection)?;
                if self.config.failpoints.stream_segment_after_segment_insert {
                    return Err(TerminalPersistenceV2Error::InvalidData(
                        "failpoint stream_segment_after_segment_insert".to_string(),
                    ));
                }

                let event = NewJournalEventRow {
                    id: event_id.clone(),
                    session_id: input.session_id.clone(),
                    pane_id: Some(input.pane_id.clone()),
                    commit_id: commit.id.clone(),
                    stream_id: stream_id.clone(),
                    event_scope_kind: "pane".to_string(),
                    event_scope_id: input.pane_id.clone(),
                    event_seq: event_seq_low,
                    event_type,
                    byte_low: Some(byte_low),
                    byte_high: Some(byte_high),
                    payload_json,
                    payload_schema_id,
                    source_event_id_hash: source_event_id_hash.clone(),
                    occurred_at_ms,
                    created_at_ms: now,
                    capture_semantics: capture_semantics.clone(),
                    trust_level: input.trust_level.unwrap_or_else(|| "captured".to_string()),
                    metadata_json: metadata_json.clone(),
                };
                insert_into(terminal_journal_events::table).values(&event).execute(connection)?;

                for (transition_index, transition) in buffer_mode_transitions.iter().enumerate() {
                    let transition_offset =
                        checked_len(transition_index + 1, "buffer mode transition offset")?;
                    let transition_event_seq =
                        event_seq_low.checked_add(transition_offset).ok_or_else(|| {
                            TerminalPersistenceV2Error::InvalidData(
                                "buffer mode transition event sequence overflow".to_string(),
                            )
                        })?;
                    let transition_byte_low =
                        byte_low.checked_add(transition.byte_offset).ok_or_else(|| {
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
                        "derived_from_event_seq": event_seq_low
                    }))?;
                    let transition_event = NewJournalEventRow {
                        id: new_id(),
                        session_id: input.session_id.clone(),
                        pane_id: Some(input.pane_id.clone()),
                        commit_id: commit.id.clone(),
                        stream_id: stream_id.clone(),
                        event_scope_kind: "pane".to_string(),
                        event_scope_id: input.pane_id.clone(),
                        event_seq: transition_event_seq,
                        event_type: "terminal_buffer_mode".to_string(),
                        byte_low: Some(transition_byte_low),
                        byte_high: Some(transition_byte_high.min(byte_high)),
                        payload_json: Some(payload_json),
                        payload_schema_id: Some(PAYLOAD_SCHEMA_JOURNAL_EVENT_V1.to_string()),
                        source_event_id_hash: None,
                        occurred_at_ms,
                        created_at_ms: now,
                        capture_semantics: capture_semantics.clone(),
                        trust_level: "parser_derived".to_string(),
                        metadata_json: Some(serde_json::to_string(&serde_json::json!({
                            "parser": "terminal_buffer_mode_detector_v1",
                            "source_segment_id": segment_id.clone()
                        }))?),
                    };
                    insert_into(terminal_journal_events::table)
                        .values(&transition_event)
                        .execute(connection)?;
                }

                let outbox = NewOutboxMessageRow {
                    id: new_id(),
                    message_kind: "pane_history_projection".to_string(),
                    dedupe_key: Some(normalize_outbox_dedupe_key(&format!(
                        "pane_history_projection:{}",
                        commit.id
                    ))),
                    state: "pending".to_string(),
                    payload_json: serde_json::to_string(&serde_json::json!({
                        "session_id": input.session_id.clone(),
                        "pane_id": input.pane_id.clone(),
                        "stream_id": stream_id.clone(),
                        "commit_id": commit.id.clone(),
                        "event_seq_low": event_seq_low,
                        "event_seq_high": final_event_seq,
                        "byte_low": byte_low,
                        "byte_high": byte_high
                    }))?,
                    attempts: 0,
                    max_attempts: 5,
                    claimed_by: None,
                    lease_token: None,
                    claimed_until_ms: None,
                    next_run_at_ms: now,
                    last_error: None,
                    created_at_ms: now,
                    updated_at_ms: now,
                };
                insert_into(terminal_outbox_messages::table).values(&outbox).execute(connection)?;

                if let (Some(source_kind), Some(source_event_id_hash)) =
                    (capture_source_kind.as_deref(), source_event_id_hash.as_deref())
                {
                    let receipt = NewCaptureReceiptRow {
                        id: new_id(),
                        session_id: input.session_id.clone(),
                        commit_id: Some(commit.id.clone()),
                        source_kind: source_kind.to_string(),
                        source_event_id_hash: source_event_id_hash.to_string(),
                        source_payload_hash: payload_checksum.clone(),
                        received_at_ms: occurred_at_ms,
                        created_at_ms: now,
                        metadata_json: metadata_json.clone(),
                    };
                    insert_into(terminal_capture_receipts::table)
                        .values(&receipt)
                        .execute(connection)?;
                }

                advance_stream_cursor(connection, &cursor.id, final_event_seq + 1, byte_high, now)?;
                diesel::update(terminal_panes::table.filter(terminal_panes::id.eq(&input.pane_id)))
                    .set(terminal_panes::last_event_seq.eq(final_event_seq))
                    .execute(connection)?;

                Ok(StreamSegmentReceipt {
                    commit_id: commit.id,
                    commit_seq: commit.commit_seq,
                    segment_id,
                    event_id,
                    event_seq_low,
                    event_seq_high,
                    byte_low,
                    byte_high,
                    checksum: payload_checksum,
                })
            });
        if let Err(error) = &append_result
            && is_storage_full_like_error(error)
        {
            let _ = self.record_storage_pressure_write_failure(
                "append_stream_segment",
                "sqlite_full",
                Some(error.to_string()),
            );
        }
        append_result
    }
}
