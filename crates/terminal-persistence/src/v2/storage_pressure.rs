use super::*;

impl TerminalPersistenceV2 {
    pub fn record_storage_pressure_event(
        &self,
        input: StoragePressureEventInput,
    ) -> Result<StoragePressureRecord, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        self.record_storage_pressure_event_with_connection(&mut connection, input)
    }

    pub(in crate::v2) fn record_storage_pressure_event_with_connection(
        &self,
        connection: &mut SqliteConnection,
        input: StoragePressureEventInput,
    ) -> Result<StoragePressureRecord, TerminalPersistenceV2Error> {
        let now = self.config.clock.now_ms();
        let row = NewStoragePressureEventRow {
            id: input.id.unwrap_or_else(new_id),
            state: input.state.unwrap_or_else(|| "ok".to_string()),
            db_file_bytes: input.db_file_bytes,
            wal_file_bytes: input.wal_file_bytes,
            disk_free_bytes: input.disk_free_bytes,
            temp_free_bytes: input.temp_free_bytes,
            quota_bytes: input.quota_bytes,
            action_taken: input.action_taken.unwrap_or_else(|| "warn_only".to_string()),
            reason: input.reason,
            created_at_ms: now,
            metadata_json: json_metadata(&input.metadata)?,
        };
        validate_storage_pressure_domain(&row.state, &row.action_taken)?;
        insert_into(terminal_storage_pressure_events::table).values(&row).execute(connection)?;
        Ok(StoragePressureRecord::from(row))
    }

    pub fn probe_storage_health(
        &self,
    ) -> Result<StoragePressureRecord, TerminalPersistenceV2Error> {
        let db_file_bytes = fs::metadata(&self.path)
            .ok()
            .map(|metadata| metadata.len())
            .map(|len| u64_to_i64(len, "database file size"))
            .transpose()?;
        let wal_path = sqlite_sidecar_path(&self.path, "-wal");
        let wal_file_bytes = fs::metadata(&wal_path)
            .ok()
            .map(|metadata| metadata.len())
            .map(|len| u64_to_i64(len, "wal file size"))
            .transpose()?;
        let classification =
            classify_storage_pressure(db_file_bytes, wal_file_bytes, self.config.storage_pressure);

        self.record_storage_pressure_event(StoragePressureEventInput {
            id: None,
            state: Some(classification.state.to_string()),
            db_file_bytes,
            wal_file_bytes,
            disk_free_bytes: None,
            temp_free_bytes: None,
            quota_bytes: None,
            action_taken: Some(classification.action_taken.to_string()),
            reason: Some(classification.reason.to_string()),
            metadata: Some(serde_json::json!({
                "db_path_hash": path_hash(&self.path),
                "wal_path_hash": path_hash(&wal_path),
                "db_warning_bytes": self.config.storage_pressure.db_warning_bytes,
                "wal_warning_bytes": self.config.storage_pressure.wal_warning_bytes,
                "db_over_budget": classification.db_over_budget,
                "wal_over_budget": classification.wal_over_budget,
                "no_silent_delete": true,
            })),
        })
    }

    pub(in crate::v2) fn record_storage_pressure_write_failure_with_connection(
        &self,
        connection: &mut SqliteConnection,
        operation: &str,
        reason: &str,
        error: Option<String>,
    ) -> Result<StoragePressureRecord, TerminalPersistenceV2Error> {
        let db_file_bytes = file_len_i64(&self.path)?;
        let wal_file_bytes = file_len_i64(&sqlite_sidecar_path(&self.path, "-wal"))?;
        self.record_storage_pressure_event_with_connection(
            connection,
            StoragePressureEventInput {
                id: None,
                state: Some("full".to_string()),
                db_file_bytes,
                wal_file_bytes,
                disk_free_bytes: None,
                temp_free_bytes: None,
                quota_bytes: None,
                action_taken: Some("fail_closed".to_string()),
                reason: Some(reason.to_string()),
                metadata: Some(serde_json::json!({
                    "operation": operation,
                    "error": error,
                    "no_silent_delete": true,
                    "canonical_history_preserved": true,
                })),
            },
        )
    }
}
