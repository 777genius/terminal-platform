use super::*;

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

    pub fn claim_next_outbox_message(
        &self,
        worker_id: &str,
        lease_ms: i64,
    ) -> Result<Option<OutboxMessageRecord>, TerminalPersistenceV2Error> {
        if lease_ms <= 0 {
            return Err(TerminalPersistenceV2Error::InvalidData(
                "outbox lease_ms must be positive".to_string(),
            ));
        }
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        connection.immediate_transaction::<_, TerminalPersistenceV2Error, _>(|connection| {
            let candidate = terminal_outbox_messages::table
                .filter(
                    terminal_outbox_messages::state
                        .eq("pending")
                        .and(terminal_outbox_messages::next_run_at_ms.le(now))
                        .or(terminal_outbox_messages::state
                            .eq("claimed")
                            .and(terminal_outbox_messages::claimed_until_ms.le(Some(now)))),
                )
                .filter(
                    terminal_outbox_messages::attempts.lt(terminal_outbox_messages::max_attempts),
                )
                .order((
                    terminal_outbox_messages::next_run_at_ms.asc(),
                    terminal_outbox_messages::created_at_ms.asc(),
                ))
                .select(OutboxMessageRow::as_select())
                .first::<OutboxMessageRow>(connection)
                .optional()?;
            let Some(candidate) = candidate else {
                return Ok(None);
            };

            let lease_token = new_id();
            let updated = diesel::update(
                terminal_outbox_messages::table
                    .filter(terminal_outbox_messages::id.eq(&candidate.id))
                    .filter(
                        terminal_outbox_messages::state.eq("pending").or(
                            terminal_outbox_messages::state
                                .eq("claimed")
                                .and(terminal_outbox_messages::claimed_until_ms.le(Some(now))),
                        ),
                    ),
            )
            .set((
                terminal_outbox_messages::state.eq("claimed"),
                terminal_outbox_messages::attempts.eq(candidate.attempts + 1),
                terminal_outbox_messages::claimed_by.eq(Some(worker_id.to_string())),
                terminal_outbox_messages::lease_token.eq(Some(lease_token)),
                terminal_outbox_messages::claimed_until_ms.eq(Some(now + lease_ms)),
                terminal_outbox_messages::last_error.eq::<Option<String>>(None),
                terminal_outbox_messages::updated_at_ms.eq(now),
            ))
            .execute(connection)?;
            if updated == 0 {
                return Ok(None);
            }

            load_outbox_message(connection, &candidate.id)?.try_into().map(Some)
        })
    }

    pub fn mark_outbox_message_done(
        &self,
        message_id: &str,
        lease_token: &str,
    ) -> Result<bool, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        let updated = diesel::update(
            terminal_outbox_messages::table
                .filter(terminal_outbox_messages::id.eq(message_id))
                .filter(terminal_outbox_messages::lease_token.eq(Some(lease_token.to_string())))
                .filter(terminal_outbox_messages::state.eq("claimed")),
        )
        .set((
            terminal_outbox_messages::state.eq("done"),
            terminal_outbox_messages::claimed_by.eq::<Option<String>>(None),
            terminal_outbox_messages::lease_token.eq::<Option<String>>(None),
            terminal_outbox_messages::claimed_until_ms.eq::<Option<i64>>(None),
            terminal_outbox_messages::updated_at_ms.eq(now),
        ))
        .execute(&mut connection)?;
        Ok(updated > 0)
    }

    pub fn fail_outbox_message(
        &self,
        message_id: &str,
        lease_token: &str,
        error: &str,
    ) -> Result<OutboxMessageRecord, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        connection.transaction::<_, TerminalPersistenceV2Error, _>(|connection| {
            let row = terminal_outbox_messages::table
                .filter(terminal_outbox_messages::id.eq(message_id))
                .filter(terminal_outbox_messages::lease_token.eq(Some(lease_token.to_string())))
                .filter(terminal_outbox_messages::state.eq("claimed"))
                .select(OutboxMessageRow::as_select())
                .first::<OutboxMessageRow>(connection)?;
            let next_state =
                if row.attempts >= row.max_attempts { "quarantined" } else { "pending" };
            let retry_delay_ms = 1_000_i64.saturating_mul(row.attempts.max(1));
            diesel::update(
                terminal_outbox_messages::table.filter(terminal_outbox_messages::id.eq(message_id)),
            )
            .set((
                terminal_outbox_messages::state.eq(next_state),
                terminal_outbox_messages::claimed_by.eq::<Option<String>>(None),
                terminal_outbox_messages::lease_token.eq::<Option<String>>(None),
                terminal_outbox_messages::claimed_until_ms.eq::<Option<i64>>(None),
                terminal_outbox_messages::next_run_at_ms.eq(now + retry_delay_ms),
                terminal_outbox_messages::last_error.eq(Some(error.to_string())),
                terminal_outbox_messages::updated_at_ms.eq(now),
            ))
            .execute(connection)?;
            load_outbox_message(connection, message_id)?.try_into()
        })
    }

    pub fn outbox_diagnostics(
        &self,
    ) -> Result<OutboxDiagnosticsRecord, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        collect_outbox_diagnostics(&mut connection, self.config.clock.now_ms())
    }
}
