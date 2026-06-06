use super::*;

pub(super) fn recover_expired_maintenance_leases(
    connection: &mut SqliteConnection,
    now: i64,
) -> Result<MaintenanceRecoverySummary, TerminalPersistenceV2Error> {
    connection.immediate_transaction::<_, TerminalPersistenceV2Error, _>(|connection| {
        let retryable_outbox = diesel::update(
            terminal_outbox_messages::table
                .filter(terminal_outbox_messages::state.eq("claimed"))
                .filter(terminal_outbox_messages::claimed_until_ms.le(Some(now)))
                .filter(
                    terminal_outbox_messages::attempts.lt(terminal_outbox_messages::max_attempts),
                ),
        )
        .set((
            terminal_outbox_messages::state.eq("pending"),
            terminal_outbox_messages::claimed_by.eq::<Option<String>>(None),
            terminal_outbox_messages::lease_token.eq::<Option<String>>(None),
            terminal_outbox_messages::claimed_until_ms.eq::<Option<i64>>(None),
            terminal_outbox_messages::next_run_at_ms.eq(now),
            terminal_outbox_messages::last_error
                .eq(Some("outbox lease expired during maintenance recovery".to_string())),
            terminal_outbox_messages::updated_at_ms.eq(now),
        ))
        .execute(connection)?;

        let exhausted_outbox = diesel::update(
            terminal_outbox_messages::table
                .filter(terminal_outbox_messages::state.eq("claimed"))
                .filter(terminal_outbox_messages::claimed_until_ms.le(Some(now)))
                .filter(
                    terminal_outbox_messages::attempts.ge(terminal_outbox_messages::max_attempts),
                ),
        )
        .set((
            terminal_outbox_messages::state.eq("quarantined"),
            terminal_outbox_messages::claimed_by.eq::<Option<String>>(None),
            terminal_outbox_messages::lease_token.eq::<Option<String>>(None),
            terminal_outbox_messages::claimed_until_ms.eq::<Option<i64>>(None),
            terminal_outbox_messages::next_run_at_ms.eq(now),
            terminal_outbox_messages::last_error
                .eq(Some("outbox lease expired after max attempts".to_string())),
            terminal_outbox_messages::updated_at_ms.eq(now),
        ))
        .execute(connection)?;

        let stale_writer_ids = terminal_writer_generations::table
            .filter(terminal_writer_generations::state.eq("active"))
            .filter(terminal_writer_generations::lease_expires_at_ms.le(now))
            .select(terminal_writer_generations::id)
            .load::<String>(connection)?;
        let stale_writers = diesel::update(
            terminal_writer_generations::table
                .filter(terminal_writer_generations::state.eq("active"))
                .filter(terminal_writer_generations::lease_expires_at_ms.le(now)),
        )
        .set((
            terminal_writer_generations::state.eq("stale"),
            terminal_writer_generations::released_at_ms.eq(Some(now)),
        ))
        .execute(connection)?;

        for writer_generation in stale_writer_ids.iter().take(stale_writers) {
            insert_clock_anchor(connection, writer_generation, now, "writer_stale_recovery")?;
        }

        Ok(MaintenanceRecoverySummary {
            stale_outbox_claims_requeued: retryable_outbox,
            stale_outbox_claims_quarantined: exhausted_outbox,
            stale_writer_generations_marked: stale_writers,
        })
    })
}

pub(super) fn map_writer_generation_insert_error(error: DieselError) -> TerminalPersistenceV2Error {
    match error {
        DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _) => {
            TerminalPersistenceV2Error::WriterAlreadyActive
        }
        other => other.into(),
    }
}
