use super::super::*;

impl TerminalPersistenceV2 {
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
}
