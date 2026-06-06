use super::{
    super::super::super::*,
    super::registration::{finish_writer_operation, upsert_history_gap_target_with_connection},
    payload::history_gap_payload,
    transaction::{HistoryGapTransaction, append_history_gap_transaction},
};

impl TerminalPersistenceV2 {
    pub fn record_history_gap_event(
        &self,
        input: HistoryGapEventInput,
    ) -> Result<JournalEventReceipt, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        self.record_history_gap_event_with_connection(&mut connection, input)
    }

    pub(crate) fn record_history_gap_event_with_connection(
        &self,
        connection: &mut SqliteConnection,
        input: HistoryGapEventInput,
    ) -> Result<JournalEventReceipt, TerminalPersistenceV2Error> {
        upsert_history_gap_target_with_connection(self, connection, &input)?;

        let lease = self.acquire_writer_generation_with_retry_on_connection(
            connection,
            "runtime-output-gap",
            60_000,
        )?;
        let append_result = self.append_history_gap_event_with_connection(
            connection,
            &input.session_id,
            &input.pane_id,
            &lease.id,
            input.skipped_events,
            input.estimated_dropped_bytes,
            &input.reason,
            input.occurred_at_ms,
        );
        let release_result = self.release_writer_generation_with_connection(connection, &lease.id);
        finish_writer_operation(append_result, release_result)
    }

    #[cfg(test)]
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
        self.append_history_gap_event_with_connection(
            &mut connection,
            session_id,
            pane_id,
            writer_generation,
            skipped_events,
            estimated_dropped_bytes,
            reason,
            occurred_at_ms,
        )
    }

    pub(crate) fn append_history_gap_event_with_connection(
        &self,
        connection: &mut SqliteConnection,
        session_id: &str,
        pane_id: &str,
        writer_generation: &str,
        skipped_events: u64,
        estimated_dropped_bytes: Option<i64>,
        reason: &str,
        occurred_at_ms: Option<i64>,
    ) -> Result<JournalEventReceipt, TerminalPersistenceV2Error> {
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
