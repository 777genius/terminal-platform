mod rows;
mod side_effects;
mod transitions;

use super::super::*;

use self::{
    rows::{
        PrimaryEventRowInput, StreamSegmentRowInput, new_primary_journal_event_row,
        new_stream_segment_row,
    },
    side_effects::{
        CaptureReceiptInput, ProjectionOutboxInput, enqueue_pane_history_projection,
        insert_capture_receipt,
    },
    transitions::{BufferModeTransitionInput, insert_buffer_mode_transition_events},
};

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

                let segment = new_stream_segment_row(StreamSegmentRowInput {
                    segment_id: &segment_id,
                    session_id: &input.session_id,
                    pane_id: &input.pane_id,
                    commit_id: &commit.id,
                    stream_id: &stream_id,
                    event_seq_low,
                    event_seq_high,
                    byte_low,
                    byte_high,
                    payload: &input.payload,
                    payload_len,
                    payload_checksum: &payload_checksum,
                    capture_semantics: &capture_semantics,
                    writer_generation: &input.writer_generation,
                    metadata_json: metadata_json.clone(),
                    now,
                });
                insert_into(terminal_stream_segments::table).values(&segment).execute(connection)?;
                if self.config.failpoints.stream_segment_after_segment_insert {
                    return Err(TerminalPersistenceV2Error::InvalidData(
                        "failpoint stream_segment_after_segment_insert".to_string(),
                    ));
                }

                let event = new_primary_journal_event_row(PrimaryEventRowInput {
                    event_id: &event_id,
                    session_id: &input.session_id,
                    pane_id: &input.pane_id,
                    commit_id: &commit.id,
                    stream_id: &stream_id,
                    event_seq: event_seq_low,
                    event_type,
                    byte_low,
                    byte_high,
                    payload_json,
                    payload_schema_id,
                    source_event_id_hash: source_event_id_hash.clone(),
                    occurred_at_ms,
                    now,
                    capture_semantics: &capture_semantics,
                    trust_level: input.trust_level.unwrap_or_else(|| "captured".to_string()),
                    metadata_json: metadata_json.clone(),
                });
                insert_into(terminal_journal_events::table).values(&event).execute(connection)?;

                insert_buffer_mode_transition_events(
                    connection,
                    BufferModeTransitionInput {
                        transitions: &buffer_mode_transitions,
                        session_id: &input.session_id,
                        pane_id: &input.pane_id,
                        commit_id: &commit.id,
                        stream_id: &stream_id,
                        segment_id: &segment_id,
                        event_seq_low,
                        byte_low,
                        byte_high,
                        occurred_at_ms,
                        now,
                        capture_semantics: &capture_semantics,
                    },
                )?;

                enqueue_pane_history_projection(
                    connection,
                    ProjectionOutboxInput {
                        session_id: &input.session_id,
                        pane_id: &input.pane_id,
                        stream_id: &stream_id,
                        commit_id: &commit.id,
                        event_seq_low,
                        event_seq_high: final_event_seq,
                        byte_low,
                        byte_high,
                        now,
                    },
                )?;

                if let (Some(source_kind), Some(source_event_id_hash)) =
                    (capture_source_kind.as_deref(), source_event_id_hash.as_deref())
                {
                    insert_capture_receipt(
                        connection,
                        CaptureReceiptInput {
                            session_id: &input.session_id,
                            commit_id: &commit.id,
                            source_kind,
                            source_event_id_hash,
                            source_payload_hash: &payload_checksum,
                            received_at_ms: occurred_at_ms,
                            created_at_ms: now,
                            metadata_json: metadata_json.clone(),
                        },
                    )?;
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
