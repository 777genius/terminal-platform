use super::super::super::*;
use super::registration::{finish_writer_operation, upsert_terminal_output_target_with_connection};

impl TerminalPersistenceV2 {
    pub fn record_terminal_output_event(
        &self,
        input: TerminalOutputEventInput,
    ) -> Result<StreamSegmentReceipt, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        self.record_terminal_output_event_with_connection(&mut connection, input)
    }

    pub(crate) fn record_terminal_output_event_with_connection(
        &self,
        connection: &mut SqliteConnection,
        input: TerminalOutputEventInput,
    ) -> Result<StreamSegmentReceipt, TerminalPersistenceV2Error> {
        upsert_terminal_output_target_with_connection(self, connection, &input)?;
        if self.is_session_private_with_connection(connection, &input.session_id)? {
            return Err(TerminalPersistenceV2Error::InvalidData(
                "private mode suppresses durable terminal output capture".to_string(),
            ));
        }

        let lease = self.acquire_writer_generation_with_retry_on_connection(
            connection,
            "runtime-output-capture",
            60_000,
        )?;
        let append_result = self.append_stream_segment_with_connection(
            connection,
            stream_segment_input(input, &lease.id),
        );
        let release_result = self.release_writer_generation_with_connection(connection, &lease.id);
        finish_writer_operation(append_result, release_result)
    }
}

fn stream_segment_input(
    input: TerminalOutputEventInput,
    writer_generation: &str,
) -> StreamSegmentInput {
    StreamSegmentInput {
        session_id: input.session_id,
        pane_id: input.pane_id,
        stream_id: None,
        writer_generation: writer_generation.to_string(),
        payload: input.payload,
        event_type: Some("terminal_output".to_string()),
        event_count: 1,
        occurred_at_ms: input.occurred_at_ms,
        capture_semantics: input.capture_semantics,
        trust_level: Some("captured".to_string()),
        payload_json: None,
        source_event_id_hash: input
            .source_sequence
            .map(|sequence| blake3_hash_text(&format!("raw-output-seq:{sequence}"))),
        metadata: Some(serde_json::json!({
            "backend_source": "runtime_capture",
            "source_sequence": input.source_sequence
        })),
    }
}
