use super::super::*;

impl TerminalPersistenceV2 {
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
}
