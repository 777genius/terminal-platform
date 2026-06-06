use super::super::*;

impl TerminalPersistenceV2 {
    pub fn enqueue_outbox_message(
        &self,
        input: OutboxMessageInput,
    ) -> Result<OutboxMessageRecord, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        let max_attempts = input.max_attempts.unwrap_or(5);
        if max_attempts <= 0 {
            return Err(TerminalPersistenceV2Error::InvalidData(
                "outbox max_attempts must be positive".to_string(),
            ));
        }
        let dedupe_key = input.dedupe_key.as_deref().map(normalize_outbox_dedupe_key);
        if let Some(dedupe_key) = dedupe_key.as_deref()
            && let Some(existing) = load_outbox_message_by_dedupe(&mut connection, dedupe_key)?
        {
            return existing.try_into();
        }

        let row = NewOutboxMessageRow {
            id: new_id(),
            message_kind: input.message_kind,
            dedupe_key,
            state: "pending".to_string(),
            payload_json: serde_json::to_string(&input.payload)?,
            attempts: 0,
            max_attempts,
            claimed_by: None,
            lease_token: None,
            claimed_until_ms: None,
            next_run_at_ms: input.next_run_at_ms.unwrap_or(now),
            last_error: None,
            created_at_ms: now,
            updated_at_ms: now,
        };
        insert_into(terminal_outbox_messages::table).values(&row).execute(&mut connection)?;
        load_outbox_message(&mut connection, &row.id)?.try_into()
    }
}
