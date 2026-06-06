use super::*;

impl TerminalPersistenceV2 {
    pub fn vacuum_into_backup(
        &self,
        target_path: impl AsRef<Path>,
    ) -> Result<BackupRecord, TerminalPersistenceV2Error> {
        let target_path = prepare_vacuum_backup_target(&self.path, target_path.as_ref())?;

        let id = new_id();
        let started_at_ms = self.config.clock.now_ms();
        let target_ref_hash = path_hash(&target_path);
        let source_db_path_hash = path_hash(&self.path);
        let mut connection = self.connection()?;
        let running = NewBackupRecordRow {
            id: id.clone(),
            backup_kind: "vacuum_into".to_string(),
            state: "running".to_string(),
            target_ref_hash: Some(target_ref_hash.clone()),
            manifest_json: None,
            checksum_algorithm: None,
            checksum: None,
            source_db_path_hash: Some(source_db_path_hash.clone()),
            started_at_ms,
            finished_at_ms: None,
            quick_check_result: None,
            error: None,
            metadata_json: None,
        };
        insert_into(terminal_backup_records::table).values(&running).execute(&mut connection)?;

        let backup_result = self.finish_vacuum_into_backup(
            &id,
            &target_path,
            started_at_ms,
            target_ref_hash,
            source_db_path_hash,
        );
        if let Err(error) = &backup_result {
            let _ = self.mark_backup_failed(&id, error.to_string());
        }
        backup_result
    }

    fn finish_vacuum_into_backup(
        &self,
        id: &str,
        target_path: &Path,
        started_at_ms: i64,
        target_ref_hash: String,
        source_db_path_hash: String,
    ) -> Result<BackupRecord, TerminalPersistenceV2Error> {
        let target_arg = target_path.to_str().ok_or_else(|| {
            TerminalPersistenceV2Error::InvalidData("backup target path is not UTF-8".to_string())
        })?;
        let mut vacuum_connection = self.connection()?;
        diesel::sql_query("VACUUM INTO ?")
            .bind::<diesel::sql_types::Text, _>(target_arg.to_string())
            .execute(&mut vacuum_connection)?;

        let checksum = blake3_hash_file(target_path)?;
        let file_bytes = u64_to_i64(fs::metadata(target_path)?.len(), "backup file size")?;
        let mut backup_connection = establish_initialized_connection(target_path, &self.config)?;
        let quick_check = run_quick_check(&mut backup_connection)?;
        let quick_check_result = quick_check.join("; ");
        if !quick_check.iter().all(|value| value == "ok") {
            return Err(TerminalPersistenceV2Error::InvalidData(format!(
                "backup quick_check failed: {quick_check_result}"
            )));
        }

        let finished_at_ms = self.config.clock.now_ms();
        let manifest = serde_json::json!({
            "backup_kind": "vacuum_into",
            "file_bytes": file_bytes,
            "target_ref_hash": target_ref_hash,
            "source_db_path_hash": source_db_path_hash,
            "checksum_algorithm": "blake3",
            "checksum": checksum,
            "quick_check_result": quick_check_result,
            "started_at_ms": started_at_ms,
            "finished_at_ms": finished_at_ms,
        });
        let manifest_json = serde_json::to_string(&manifest)?;

        let mut connection = self.connection()?;
        diesel::update(terminal_backup_records::table.filter(terminal_backup_records::id.eq(id)))
            .set((
                terminal_backup_records::state.eq("succeeded"),
                terminal_backup_records::manifest_json.eq(Some(manifest_json.clone())),
                terminal_backup_records::checksum_algorithm.eq(Some("blake3".to_string())),
                terminal_backup_records::checksum.eq(Some(checksum.clone())),
                terminal_backup_records::finished_at_ms.eq(Some(finished_at_ms)),
                terminal_backup_records::quick_check_result.eq(Some(quick_check_result.clone())),
                terminal_backup_records::error.eq::<Option<String>>(None),
            ))
            .execute(&mut connection)?;

        Ok(BackupRecord {
            id: id.to_string(),
            backup_kind: "vacuum_into".to_string(),
            state: "succeeded".to_string(),
            target_ref_hash: Some(target_ref_hash),
            manifest_json: Some(manifest),
            checksum_algorithm: Some("blake3".to_string()),
            checksum: Some(checksum),
            source_db_path_hash: Some(source_db_path_hash),
            started_at_ms,
            finished_at_ms: Some(finished_at_ms),
            quick_check_result: Some(quick_check_result),
            error: None,
        })
    }

    fn mark_backup_failed(
        &self,
        id: &str,
        error: String,
    ) -> Result<(), TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        diesel::update(terminal_backup_records::table.filter(terminal_backup_records::id.eq(id)))
            .set((
                terminal_backup_records::state.eq("failed"),
                terminal_backup_records::finished_at_ms.eq(Some(self.config.clock.now_ms())),
                terminal_backup_records::error.eq(Some(error)),
            ))
            .execute(&mut connection)?;
        Ok(())
    }
}
