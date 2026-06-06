use super::*;

impl TerminalPersistenceV2 {
    pub fn acquire_writer_generation(
        &self,
        process_id: impl Into<String>,
        lease_ms: i64,
    ) -> Result<WriterGenerationLease, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        self.acquire_writer_generation_with_connection(&mut connection, process_id, lease_ms)
    }

    pub(in crate::v2) fn acquire_writer_generation_with_connection(
        &self,
        connection: &mut SqliteConnection,
        process_id: impl Into<String>,
        lease_ms: i64,
    ) -> Result<WriterGenerationLease, TerminalPersistenceV2Error> {
        if lease_ms <= 0 {
            return Err(TerminalPersistenceV2Error::InvalidData(
                "writer lease_ms must be positive".to_string(),
            ));
        }

        let now = self.config.clock.now_ms();
        let process_id = process_id.into();
        let id = new_id();
        let lease_token = new_id();

        connection.transaction::<_, TerminalPersistenceV2Error, _>(|connection| {
            diesel::update(
                terminal_writer_generations::table.filter(
                    terminal_writer_generations::state
                        .eq("active")
                        .and(terminal_writer_generations::lease_expires_at_ms.le(now)),
                ),
            )
            .set((
                terminal_writer_generations::state.eq("stale"),
                terminal_writer_generations::released_at_ms.eq(Some(now)),
            ))
            .execute(connection)?;

            let active = terminal_writer_generations::table
                .filter(terminal_writer_generations::state.eq("active"))
                .select(WriterGenerationRow::as_select())
                .first::<WriterGenerationRow>(connection)
                .optional()?;
            if active.is_some() {
                return Err(TerminalPersistenceV2Error::WriterAlreadyActive);
            }

            let row = NewWriterGenerationRow {
                id: id.clone(),
                process_id: process_id.clone(),
                lease_token: lease_token.clone(),
                state: "active".to_string(),
                acquired_at_ms: now,
                heartbeat_at_ms: now,
                lease_expires_at_ms: now + lease_ms,
                released_at_ms: None,
                metadata_json: None,
            };
            insert_into(terminal_writer_generations::table)
                .values(&row)
                .execute(connection)
                .map_err(map_writer_generation_insert_error)?;
            insert_clock_anchor(connection, &id, now, "writer_acquire")?;

            Ok(())
        })?;

        Ok(WriterGenerationLease {
            id,
            process_id,
            lease_token,
            lease_expires_at_ms: now + lease_ms,
        })
    }

    pub(in crate::v2) fn acquire_writer_generation_with_retry_on_connection(
        &self,
        connection: &mut SqliteConnection,
        process_id: &str,
        lease_ms: i64,
    ) -> Result<WriterGenerationLease, TerminalPersistenceV2Error> {
        const ATTEMPTS: usize = 40;
        const BACKOFF: Duration = Duration::from_millis(25);

        for attempt in 0..ATTEMPTS {
            match self.acquire_writer_generation_with_connection(connection, process_id, lease_ms) {
                Ok(lease) => return Ok(lease),
                Err(TerminalPersistenceV2Error::WriterAlreadyActive) if attempt + 1 < ATTEMPTS => {
                    thread::sleep(BACKOFF);
                }
                Err(error) => return Err(error),
            }
        }

        Err(TerminalPersistenceV2Error::WriterAlreadyActive)
    }

    pub fn heartbeat_writer_generation(
        &self,
        writer_generation: &str,
        lease_ms: i64,
    ) -> Result<(), TerminalPersistenceV2Error> {
        if lease_ms <= 0 {
            return Err(TerminalPersistenceV2Error::InvalidData(
                "writer lease_ms must be positive".to_string(),
            ));
        }
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        connection.transaction::<_, TerminalPersistenceV2Error, _>(|connection| {
            let updated = diesel::update(
                terminal_writer_generations::table
                    .filter(terminal_writer_generations::id.eq(writer_generation))
                    .filter(terminal_writer_generations::state.eq("active")),
            )
            .set((
                terminal_writer_generations::heartbeat_at_ms.eq(now),
                terminal_writer_generations::lease_expires_at_ms.eq(now + lease_ms),
            ))
            .execute(connection)?;
            if updated == 0 {
                return Err(TerminalPersistenceV2Error::InvalidData(
                    "active writer generation not found for heartbeat".to_string(),
                ));
            }
            insert_clock_anchor(connection, writer_generation, now, "writer_heartbeat")?;
            Ok(())
        })?;
        Ok(())
    }

    pub fn release_writer_generation(
        &self,
        writer_generation: &str,
    ) -> Result<(), TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        self.release_writer_generation_with_connection(&mut connection, writer_generation)
    }

    pub(in crate::v2) fn release_writer_generation_with_connection(
        &self,
        connection: &mut SqliteConnection,
        writer_generation: &str,
    ) -> Result<(), TerminalPersistenceV2Error> {
        let now = self.config.clock.now_ms();
        connection.transaction::<_, TerminalPersistenceV2Error, _>(|connection| {
            let updated = diesel::update(
                terminal_writer_generations::table
                    .filter(terminal_writer_generations::id.eq(writer_generation))
                    .filter(terminal_writer_generations::state.eq("active")),
            )
            .set((
                terminal_writer_generations::state.eq("released"),
                terminal_writer_generations::released_at_ms.eq(Some(now)),
            ))
            .execute(connection)?;
            if updated == 0 {
                return Err(TerminalPersistenceV2Error::InvalidData(
                    "active writer generation not found for release".to_string(),
                ));
            }
            insert_clock_anchor(connection, writer_generation, now, "writer_release")?;
            Ok(())
        })?;
        Ok(())
    }
}
