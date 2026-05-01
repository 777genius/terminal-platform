use super::super::*;

impl TerminalPersistenceV2 {
    pub fn record_restore_drill(
        &self,
        session_id: &str,
        plan: &RestorePlan,
        result: &str,
        duration_ms: Option<i64>,
        error: Option<&str>,
    ) -> Result<String, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        let id = new_id();
        let evidence_json = Some(serde_json::to_string(&plan.evidence)?);
        let row = NewRestoreDrillRow {
            id: id.clone(),
            session_id: session_id.to_string(),
            drill_kind: "restore_plan".to_string(),
            result: result.to_string(),
            restore_guarantee_level: plan.guarantee_level.as_str().to_string(),
            checked_at_ms: now,
            duration_ms,
            source_snapshot_id: plan.latest_screen_snapshot_id.clone(),
            evidence_json,
            error: error.map(ToOwned::to_owned),
            metadata_json: None,
        };
        insert_into(terminal_restore_drills::table).values(&row).execute(&mut connection)?;
        Ok(id)
    }

    pub fn run_restore_drill(
        &self,
        session_id: &str,
    ) -> Result<RestoreDrillRecord, TerminalPersistenceV2Error> {
        let started_at_ms = self.config.clock.now_ms();
        let plan = self.restore_plan(session_id)?;
        let mut connection = self.connection()?;
        connection.immediate_transaction::<_, TerminalPersistenceV2Error, _>(|connection| {
            let validation = validate_history_checksums(connection, Some(session_id))?;
            let replay_safety = collect_restore_replay_safety(connection, session_id)?;
            let finished_at_ms = self.config.clock.now_ms();
            let result = if validation.has_failures() {
                "failed"
            } else {
                match &plan.guarantee_level {
                    RestoreGuaranteeLevel::BasicHistory
                    | RestoreGuaranteeLevel::VisualSnapshotOnly => "passed",
                    RestoreGuaranteeLevel::RawStreamReplay
                    | RestoreGuaranteeLevel::LiveMuxAttach => "passed",
                    RestoreGuaranteeLevel::DegradedHistory => "degraded",
                    RestoreGuaranteeLevel::None => "skipped",
                }
            };
            let error = validation.has_failures().then(|| validation.summary());
            let mut evidence = plan.evidence.clone();
            evidence.extend(validation.to_restore_evidence());
            evidence.extend(replay_safety.to_restore_evidence());
            let evidence_json = Some(serde_json::to_string(&evidence)?);
            let metadata_json = Some(serde_json::to_string(&serde_json::json!({
                "started_at_ms": started_at_ms,
                "validation": validation.to_json(),
                "replay_safety": replay_safety,
            }))?);
            let id = new_id();
            let row = NewRestoreDrillRow {
                id: id.clone(),
                session_id: session_id.to_string(),
                drill_kind: "restore_drill".to_string(),
                result: result.to_string(),
                restore_guarantee_level: plan.guarantee_level.as_str().to_string(),
                checked_at_ms: finished_at_ms,
                duration_ms: Some((finished_at_ms - started_at_ms).max(0)),
                source_snapshot_id: plan.latest_screen_snapshot_id.clone(),
                evidence_json,
                error: error.clone(),
                metadata_json,
            };
            insert_into(terminal_restore_drills::table).values(&row).execute(connection)?;
            persist_history_validation_health_records(
                connection,
                Some(session_id),
                &validation,
                finished_at_ms,
                Some(&id),
            )?;

            Ok(RestoreDrillRecord {
                id,
                session_id: session_id.to_string(),
                drill_kind: "restore_drill".to_string(),
                result: result.to_string(),
                restore_guarantee_level: plan.guarantee_level.as_str().to_string(),
                checked_at_ms: finished_at_ms,
                duration_ms: Some((finished_at_ms - started_at_ms).max(0)),
                source_snapshot_id: plan.latest_screen_snapshot_id.clone(),
                error,
            })
        })
    }

    pub fn restore_replay_safety_diagnostics(
        &self,
        session_id: &str,
    ) -> Result<RestoreReplaySafetyRecord, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        collect_restore_replay_safety(&mut connection, session_id)
    }
}
