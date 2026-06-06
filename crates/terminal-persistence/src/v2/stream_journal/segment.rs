mod rows;
mod side_effects;
mod transaction;
mod transitions;

use super::super::*;

use self::transaction::{AppendStreamSegmentTransaction, append_stream_segment_transaction};

impl TerminalPersistenceV2 {
    pub fn append_stream_segment(
        &self,
        input: StreamSegmentInput,
    ) -> Result<StreamSegmentReceipt, TerminalPersistenceV2Error> {
        validate_stream_segment_input(&input)?;
        let mut connection = self.connection()?;
        self.append_stream_segment_with_connection(&mut connection, input)
    }

    pub(crate) fn append_stream_segment_with_connection(
        &self,
        connection: &mut SqliteConnection,
        input: StreamSegmentInput,
    ) -> Result<StreamSegmentReceipt, TerminalPersistenceV2Error> {
        validate_stream_segment_input(&input)?;
        if self.config.failpoints.stream_segment_before_transaction_storage_full {
            self.record_storage_pressure_write_failure_with_connection(
                connection,
                "append_stream_segment",
                "synthetic_sqlite_full",
                None,
            )?;
            return Err(TerminalPersistenceV2Error::InvalidData(
                "failpoint stream_segment_before_transaction_storage_full".to_string(),
            ));
        }

        let now = self.config.clock.now_ms();
        let occurred_at_ms = input.occurred_at_ms.unwrap_or(now);
        let stream_id = input.stream_id.clone().unwrap_or_else(|| DEFAULT_STREAM_ID.to_string());
        let payload_len = checked_len(input.payload.len(), "payload length")?;
        let payload_checksum = blake3_hash_bytes(&input.payload);
        let metadata_json = json_metadata(&input.metadata)?;
        let source_event_id_hash = input.source_event_id_hash.clone();
        let capture_source_kind = source_event_id_hash
            .as_ref()
            .map(|_| stream_capture_source_kind(&input.pane_id, &stream_id));
        let buffer_mode_transitions = detect_buffer_mode_transitions(&input.payload);
        let fail_after_segment_insert = self.config.failpoints.stream_segment_after_segment_insert;

        let append_result =
            connection.transaction::<_, TerminalPersistenceV2Error, _>(|connection| {
                append_stream_segment_transaction(
                    connection,
                    AppendStreamSegmentTransaction {
                        input: &input,
                        stream_id: &stream_id,
                        payload_len,
                        payload_checksum: &payload_checksum,
                        metadata_json: metadata_json.clone(),
                        source_event_id_hash: source_event_id_hash.clone(),
                        capture_source_kind: capture_source_kind.clone(),
                        buffer_mode_transitions: &buffer_mode_transitions,
                        occurred_at_ms,
                        now,
                        fail_after_segment_insert,
                    },
                )
            });
        if let Err(error) = &append_result
            && is_storage_full_like_error(error)
        {
            let _ = self.record_storage_pressure_write_failure_with_connection(
                connection,
                "append_stream_segment",
                "sqlite_full",
                Some(error.to_string()),
            );
        }
        append_result
    }
}

fn validate_stream_segment_input(
    input: &StreamSegmentInput,
) -> Result<(), TerminalPersistenceV2Error> {
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
    Ok(())
}
