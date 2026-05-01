use super::super::*;

impl TerminalPersistenceV2 {
    pub fn run_integrity_check(&self) -> Result<IntegrityCheckRecord, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let checked_at_ms = self.config.clock.now_ms();
        connection.immediate_transaction::<_, TerminalPersistenceV2Error, _>(|connection| {
            let quick_check = run_quick_check(connection)?;
            let foreign_key_violations = run_foreign_key_check(connection)?;
            let validation = validate_history_checksums(connection, None)?;
            let result = if quick_check.iter().all(|value| value == "ok")
                && foreign_key_violations.is_empty()
                && !validation.has_failures()
            {
                "passed"
            } else {
                "failed"
            };
            let details = serde_json::json!({
                "quick_check": quick_check,
                "foreign_key_violations": foreign_key_violations,
                "history_validation": validation.to_json(),
            });
            let error = (result != "passed").then(|| {
                format!(
                    "quick_check={}, foreign_key_violations={}, history_validation_failures={}, checksum_failures={}",
                    details["quick_check"],
                    details["foreign_key_violations"].as_array().map_or(0, Vec::len),
                    validation.failure_count(),
                    validation.checksum_failure_count()
                )
            });
            let id = new_id();
            let row = NewIntegrityCheckRow {
                id: id.clone(),
                check_kind: "sqlite_and_history_invariants".to_string(),
                scope_kind: "database".to_string(),
                scope_ref: None,
                result: result.to_string(),
                checked_at_ms,
                details_json: Some(serde_json::to_string(&details)?),
                error: error.clone(),
                metadata_json: None,
            };
            insert_into(terminal_integrity_checks::table).values(&row).execute(connection)?;
            persist_history_validation_health_records(
                connection,
                None,
                &validation,
                checked_at_ms,
                Some(&id),
            )?;

            Ok(IntegrityCheckRecord {
                id,
                check_kind: "sqlite_and_history_invariants".to_string(),
                scope_kind: "database".to_string(),
                scope_ref: None,
                result: result.to_string(),
                checked_at_ms,
                details_json: Some(details),
                error,
            })
        })
    }

    pub fn list_open_data_health_records(
        &self,
        session_id: Option<&str>,
    ) -> Result<Vec<DataHealthRecord>, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let mut query = terminal_data_health_records::table
            .filter(terminal_data_health_records::action_state.ne("resolved"))
            .filter(terminal_data_health_records::action_state.ne("ignored"))
            .into_boxed();
        if let Some(session_id) = session_id {
            query = query.filter(terminal_data_health_records::session_id.eq(session_id));
        }
        query
            .order(terminal_data_health_records::detected_at_ms.desc())
            .select(DataHealthRecordRow::as_select())
            .load::<DataHealthRecordRow>(&mut connection)?
            .into_iter()
            .map(TryInto::try_into)
            .collect()
    }
}
