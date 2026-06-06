use super::*;

impl TerminalPersistenceV2 {
    pub fn record_persistence_fault_health_record(
        &self,
        input: PersistenceFaultHealthRecordInput,
    ) -> Result<String, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        self.record_persistence_fault_health_record_with_connection(&mut connection, input)
    }

    pub(crate) fn record_persistence_fault_health_record_with_connection(
        &self,
        connection: &mut SqliteConnection,
        input: PersistenceFaultHealthRecordInput,
    ) -> Result<String, TerminalPersistenceV2Error> {
        let now = self.config.clock.now_ms();
        let affected_ref = format!(
            "persistence_fault:{}:{}:{}",
            input.session_id.as_deref().unwrap_or("global"),
            input.pane_id.as_deref().unwrap_or("session"),
            input.operation
        );
        let existing = terminal_data_health_records::table
            .filter(terminal_data_health_records::affected_ref.eq(Some(affected_ref.clone())))
            .filter(terminal_data_health_records::detection_kind.eq("manual"))
            .filter(terminal_data_health_records::action_state.ne("resolved"))
            .filter(terminal_data_health_records::action_state.ne("ignored"))
            .select(DataHealthRecordRow::as_select())
            .first::<DataHealthRecordRow>(connection)
            .optional()?;
        if let Some(existing) = existing {
            return Ok(existing.id);
        }

        let metadata = match (input.metadata, input.error_kind) {
            (Some(Value::Object(mut metadata)), Some(error_kind)) => {
                metadata.insert("error_kind".to_string(), Value::String(error_kind));
                Some(Value::Object(metadata))
            }
            (Some(metadata), Some(error_kind)) => Some(serde_json::json!({
                "metadata": metadata,
                "error_kind": error_kind
            })),
            (metadata, None) => metadata,
            (None, Some(error_kind)) => Some(serde_json::json!({ "error_kind": error_kind })),
        };
        let details_json = Some(serde_json::to_string(&serde_json::json!({
            "operation": input.operation,
            "detail": input.detail,
            "source": "runtime_history_persistence",
        }))?);
        let row = NewDataHealthRecordRow {
            id: new_id(),
            session_id: input.session_id,
            pane_id: input.pane_id,
            detection_kind: "manual".to_string(),
            severity: "error".to_string(),
            first_bad_event_seq: None,
            affected_ref: Some(affected_ref),
            action_state: "open".to_string(),
            detected_at_ms: now,
            resolved_at_ms: None,
            details_json,
            metadata_json: json_metadata(&metadata)?,
        };
        let id = row.id.clone();
        insert_into(terminal_data_health_records::table).values(&row).execute(connection)?;
        Ok(id)
    }

    pub fn compression_diagnostics(
        &self,
    ) -> Result<CompressionDiagnosticsRecord, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        collect_compression_diagnostics(&mut connection, self.config.clock.now_ms())
    }

    pub fn retention_diagnostics(
        &self,
        selected_policy_id: Option<&str>,
    ) -> Result<RetentionDiagnosticsRecord, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        collect_retention_diagnostics(
            &mut connection,
            self.config.clock.now_ms(),
            selected_policy_id,
        )
    }
}
