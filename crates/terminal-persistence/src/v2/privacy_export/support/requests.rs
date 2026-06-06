use super::super::super::*;

impl TerminalPersistenceV2 {
    pub fn create_support_bundle(
        &self,
        input: SupportBundleInput,
    ) -> Result<SupportBundleRecord, TerminalPersistenceV2Error> {
        if input.include_raw {
            self.ensure_raw_history_export_enabled()?;
        }
        if let Some(output_ref) = input.output_ref.as_deref() {
            validate_external_artifact_ref(output_ref)?;
            validate_external_artifact_target_ref(output_ref, &self.path)?;
        }
        let mut connection = self.connection()?;
        if input.include_raw {
            ensure_no_open_critical_health_records(&mut connection, None, "raw support bundle")?;
        }
        let now = self.config.clock.now_ms();
        let manifest = privacy_manifest("support_bundle", input.include_raw, None);
        let row = NewSupportBundleRow {
            id: input.id.unwrap_or_else(new_id),
            scope_json: serde_json::to_string(&input.scope)?,
            state: "pending".to_string(),
            redaction_profile_id: input
                .redaction_profile_id
                .or_else(|| Some("default".to_string())),
            include_raw: bool_to_int(input.include_raw),
            requested_at_ms: now,
            completed_at_ms: None,
            manifest_json: Some(serde_json::to_string(&manifest)?),
            output_ref_hash: input.output_ref.map(|value| blake3_hash_text(&value)),
            error: None,
            metadata_json: json_metadata(&input.metadata)?,
        };
        insert_into(terminal_support_bundles::table).values(&row).execute(&mut connection)?;
        Ok(SupportBundleRecord::try_from(row)?)
    }
}
