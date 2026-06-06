use super::*;

impl TerminalPersistenceV2 {
    pub fn run_maintenance(
        &self,
        input: MaintenanceRunInput,
    ) -> Result<MaintenanceRunRecord, TerminalPersistenceV2Error> {
        let id = input.id.unwrap_or_else(new_id);
        let started_at_ms = self.config.clock.now_ms();
        let run_kind = input.run_kind.unwrap_or_else(|| "scheduled_maintenance".to_string());
        let metadata_json = json_metadata(&input.metadata)?;
        let selected_policy_id = input.selected_policy_id.clone();
        let mut connection = self.connection()?;
        let row = NewMaintenanceRunRow {
            id: id.clone(),
            run_kind: run_kind.clone(),
            state: "running".to_string(),
            selected_policy_id: selected_policy_id.clone(),
            started_at_ms,
            finished_at_ms: None,
            summary_json: None,
            error: None,
            metadata_json,
        };
        insert_into(terminal_maintenance_runs::table).values(&row).execute(&mut connection)?;

        let run_result = self.finish_maintenance_run(
            &id,
            &run_kind,
            started_at_ms,
            input.run_wal_checkpoint,
            input.run_optimize,
            selected_policy_id.as_deref(),
        );
        if let Err(error) = &run_result {
            let _ = self.mark_maintenance_failed(&id, error.to_string());
        }
        run_result
    }

    fn finish_maintenance_run(
        &self,
        id: &str,
        run_kind: &str,
        started_at_ms: i64,
        run_wal_checkpoint: bool,
        run_optimize: bool,
        selected_policy_id: Option<&str>,
    ) -> Result<MaintenanceRunRecord, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let recovery = recover_expired_maintenance_leases(&mut connection, started_at_ms)?;
        let outbox_diagnostics = collect_outbox_diagnostics(&mut connection, started_at_ms)?;
        let compression_diagnostics =
            collect_compression_diagnostics(&mut connection, started_at_ms)?;
        let retention_diagnostics =
            collect_retention_diagnostics(&mut connection, started_at_ms, selected_policy_id)?;
        let wal_checkpoint = if run_wal_checkpoint {
            Some(run_passive_wal_checkpoint(&mut connection)?)
        } else {
            None
        };
        if run_optimize {
            connection.batch_execute("PRAGMA optimize;")?;
        }

        let db_file_bytes = file_len_i64(&self.path)?;
        let wal_file_bytes = file_len_i64(&sqlite_sidecar_path(&self.path, "-wal"))?;
        let finished_at_ms = self.config.clock.now_ms();
        let summary = serde_json::json!({
            "run_kind": run_kind,
            "wal_checkpoint": wal_checkpoint,
            "optimize": {
                "ran": run_optimize,
                "mode": "pragma_optimize"
            },
            "recovery": {
                "checked_at_ms": started_at_ms,
                "stale_outbox_claims_requeued": recovery.stale_outbox_claims_requeued,
                "stale_outbox_claims_quarantined": recovery.stale_outbox_claims_quarantined,
                "stale_writer_generations_marked": recovery.stale_writer_generations_marked
            },
            "outbox": outbox_diagnostics,
            "compression": compression_diagnostics,
            "retention": retention_diagnostics,
            "storage": {
                "db_file_bytes": db_file_bytes,
                "wal_file_bytes": wal_file_bytes,
                "no_silent_delete": true
            },
            "duration_ms": (finished_at_ms - started_at_ms).max(0)
        });
        diesel::update(
            terminal_maintenance_runs::table.filter(terminal_maintenance_runs::id.eq(id)),
        )
        .set((
            terminal_maintenance_runs::state.eq("succeeded"),
            terminal_maintenance_runs::finished_at_ms.eq(Some(finished_at_ms)),
            terminal_maintenance_runs::summary_json.eq(Some(serde_json::to_string(&summary)?)),
            terminal_maintenance_runs::error.eq::<Option<String>>(None),
        ))
        .execute(&mut connection)?;
        load_maintenance_run(&mut connection, id)?.try_into()
    }

    fn mark_maintenance_failed(
        &self,
        id: &str,
        error: String,
    ) -> Result<(), TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        diesel::update(
            terminal_maintenance_runs::table.filter(terminal_maintenance_runs::id.eq(id)),
        )
        .set((
            terminal_maintenance_runs::state.eq("failed"),
            terminal_maintenance_runs::finished_at_ms.eq(Some(self.config.clock.now_ms())),
            terminal_maintenance_runs::error.eq(Some(error)),
        ))
        .execute(&mut connection)?;
        Ok(())
    }
}
