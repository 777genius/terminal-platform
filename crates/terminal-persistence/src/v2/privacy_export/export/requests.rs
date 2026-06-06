use super::super::super::*;

impl TerminalPersistenceV2 {
    pub fn create_export_request(
        &self,
        input: ExportRequestInput,
    ) -> Result<ExportRequestRecord, TerminalPersistenceV2Error> {
        if input.include_raw {
            self.ensure_raw_history_export_enabled()?;
        }
        let mut connection = self.connection()?;
        if input.include_raw {
            ensure_no_open_critical_health_records(
                &mut connection,
                input.session_id.as_deref(),
                "raw export",
            )?;
        }
        let now = self.config.clock.now_ms();
        let manifest = privacy_manifest("export", input.include_raw, input.session_id.as_deref());
        let row = NewExportRequestRow {
            id: input.id.unwrap_or_else(new_id),
            session_id: input.session_id,
            export_kind: input.export_kind.unwrap_or_else(|| "redacted_logical".to_string()),
            state: "pending".to_string(),
            redaction_profile_id: input
                .redaction_profile_id
                .or_else(|| Some("default".to_string())),
            include_raw: bool_to_int(input.include_raw),
            approved_at_ms: None,
            requested_at_ms: now,
            completed_at_ms: None,
            manifest_json: Some(serde_json::to_string(&manifest)?),
            output_ref_hash: input.output_ref.map(|value| blake3_hash_text(&value)),
            error: None,
            metadata_json: json_metadata(&input.metadata)?,
        };
        insert_into(terminal_export_requests::table).values(&row).execute(&mut connection)?;
        Ok(ExportRequestRecord::try_from(row)?)
    }
}
