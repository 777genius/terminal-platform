use super::super::*;

impl TerminalPersistenceV2 {
    pub fn record_external_artifact(
        &self,
        input: ExternalArtifactInput,
    ) -> Result<ExternalArtifactRecord, TerminalPersistenceV2Error> {
        validate_external_artifact_domain(
            &input.artifact_kind,
            input.state.as_deref(),
            input.encryption_state.as_deref(),
        )?;
        validate_external_artifact_ref(&input.artifact_ref)?;
        validate_external_artifact_target_ref(&input.artifact_ref, &self.path)?;
        if let Some(size_bytes) = input.size_bytes
            && size_bytes < 0
        {
            return Err(TerminalPersistenceV2Error::InvalidData(
                "external artifact size_bytes must not be negative".to_string(),
            ));
        }

        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        let row = NewExternalArtifactRow {
            id: input.id.unwrap_or_else(new_id),
            artifact_kind: input.artifact_kind,
            artifact_ref_hash: blake3_hash_text(&input.artifact_ref),
            state: input.state.unwrap_or_else(|| "planned".to_string()),
            encryption_state: input.encryption_state.unwrap_or_else(|| "plaintext".to_string()),
            key_ref: input.key_ref,
            checksum_algorithm: input.checksum_algorithm,
            checksum: input.checksum,
            size_bytes: input.size_bytes,
            created_at_ms: now,
            verified_at_ms: input.verified_at_ms,
            metadata_json: json_metadata(&input.metadata)?,
        };
        insert_into(terminal_external_artifacts::table).values(&row).execute(&mut connection)?;
        Ok(ExternalArtifactRecord::try_from(row)?)
    }
}
