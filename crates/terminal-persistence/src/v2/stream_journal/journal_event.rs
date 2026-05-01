use super::super::*;

impl TerminalPersistenceV2 {
    pub fn append_journal_event(
        &self,
        input: JournalEventInput,
    ) -> Result<JournalEventReceipt, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        let occurred_at_ms = input.occurred_at_ms.unwrap_or(now);
        let stream_id = input.stream_id.unwrap_or_else(|| DEFAULT_STREAM_ID.to_string());
        let payload_json = input.payload_json.as_ref().map(serde_json::to_string).transpose()?;
        let payload_schema_id = payload_json
            .as_ref()
            .map(|_| payload_schema_id_for_journal_event(&input.event_type).to_string());
        let metadata_json = json_metadata(&input.metadata)?;

        connection.transaction::<_, TerminalPersistenceV2Error, _>(|connection| {
            ensure_active_writer(connection, &input.writer_generation, now)?;
            let capture_semantics =
                input.capture_semantics.unwrap_or_else(|| "raw_vt_stream".to_string());
            validate_capture_semantics_domain(&capture_semantics)?;
            let commit = allocate_commit(
                connection,
                &input.session_id,
                input.commit_kind.as_deref().unwrap_or("journal_event"),
                &input.writer_generation,
                occurred_at_ms,
                now,
                None,
            )?;
            let scope = event_scope(&input.session_id, input.pane_id.as_deref());
            let event_seq = if let Some(pane_id) = input.pane_id.as_deref() {
                let cursor =
                    load_stream_cursor(connection, &input.session_id, pane_id, &stream_id)?;
                advance_stream_cursor(
                    connection,
                    &cursor.id,
                    cursor.next_event_seq + 1,
                    cursor.next_byte_seq,
                    now,
                )?;
                diesel::update(terminal_panes::table.filter(terminal_panes::id.eq(pane_id)))
                    .set(terminal_panes::last_event_seq.eq(cursor.next_event_seq))
                    .execute(connection)?;
                cursor.next_event_seq
            } else {
                commit.commit_seq
            };
            let event_id = new_id();
            let row = NewJournalEventRow {
                id: event_id.clone(),
                session_id: input.session_id,
                pane_id: input.pane_id,
                commit_id: commit.id.clone(),
                stream_id,
                event_scope_kind: scope.kind,
                event_scope_id: scope.id,
                event_seq,
                event_type: input.event_type,
                byte_low: None,
                byte_high: None,
                payload_json,
                payload_schema_id,
                source_event_id_hash: input.source_event_id_hash,
                occurred_at_ms,
                created_at_ms: now,
                capture_semantics,
                trust_level: input.trust_level.unwrap_or_else(|| "captured".to_string()),
                metadata_json,
            };
            insert_into(terminal_journal_events::table).values(&row).execute(connection)?;

            Ok(JournalEventReceipt {
                commit_id: commit.id,
                commit_seq: commit.commit_seq,
                event_id,
                event_seq,
            })
        })
    }
}
