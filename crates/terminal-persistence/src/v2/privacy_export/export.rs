use super::super::*;

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

    pub fn approve_export_request(
        &self,
        input: ExportApprovalInput,
    ) -> Result<ExportRequestRecord, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        connection.transaction::<_, TerminalPersistenceV2Error, _>(|connection| {
            let request = load_export_request(connection, &input.export_request_id)?;
            if request.state == "succeeded" || request.state == "failed" {
                return Err(TerminalPersistenceV2Error::InvalidData(format!(
                    "export request cannot be approved from state {}",
                    request.state
                )));
            }

            let metadata_json = merge_json_field(
                request.metadata_json.as_deref(),
                "approval",
                serde_json::json!({
                    "approved_at_ms": now,
                    "approver_ref_hash": input.approver_ref.as_ref().map(|value| blake3_hash_text(value)),
                    "metadata": input.metadata,
                }),
            )?;

            diesel::update(
                terminal_export_requests::table
                    .filter(terminal_export_requests::id.eq(&input.export_request_id)),
            )
            .set((
                terminal_export_requests::state.eq("approved"),
                terminal_export_requests::approved_at_ms.eq(Some(now)),
                terminal_export_requests::metadata_json.eq(metadata_json),
            ))
            .execute(connection)?;

            ExportRequestRecord::try_from(load_export_request(connection, &input.export_request_id)?)
        })
    }

    pub fn verify_export_artifact(
        &self,
        input: ExportArtifactVerificationInput,
    ) -> Result<ExportArtifactVerificationRecord, TerminalPersistenceV2Error> {
        validate_external_artifact_ref(&input.artifact_ref)?;
        validate_external_artifact_target_ref(&input.artifact_ref, &self.path)?;
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        connection.transaction::<_, TerminalPersistenceV2Error, _>(|connection| {
            let request = load_export_request(connection, &input.export_request_id)?;
            let artifact_ref_hash = blake3_hash_text(&input.artifact_ref);
            if request.output_ref_hash.as_deref() != Some(artifact_ref_hash.as_str()) {
                return Err(TerminalPersistenceV2Error::InvalidData(
                    "export artifact ref does not match request output_ref hash".to_string(),
                ));
            }
            if request.include_raw != 0 && request.approved_at_ms.is_none() {
                return Err(TerminalPersistenceV2Error::InvalidData(
                    "raw export must be explicitly approved before artifact verification"
                        .to_string(),
                ));
            }

            let artifact = terminal_external_artifacts::table
                .filter(terminal_external_artifacts::artifact_ref_hash.eq(&artifact_ref_hash))
                .select(ExternalArtifactRow::as_select())
                .first::<ExternalArtifactRow>(connection)?;
            if artifact.artifact_kind != "export_file" {
                return Err(TerminalPersistenceV2Error::InvalidData(format!(
                    "export artifact verification requires export_file artifact, got {}",
                    artifact.artifact_kind
                )));
            }
            if artifact.state != "available" && artifact.state != "verified" {
                return Err(TerminalPersistenceV2Error::InvalidData(format!(
                    "export artifact must be available or verified, got {}",
                    artifact.state
                )));
            }

            let raw_export = request.include_raw != 0;
            let encrypted_required = raw_export || input.require_encrypted;
            if encrypted_required && artifact.encryption_state != "encrypted" {
                return Err(TerminalPersistenceV2Error::InvalidData(
                    "raw or encrypted-required export must complete into an encrypted artifact"
                        .to_string(),
                ));
            }
            if encrypted_required && artifact.key_ref.is_none() {
                return Err(TerminalPersistenceV2Error::InvalidData(
                    "encrypted export artifact must reference an opaque key id".to_string(),
                ));
            }
            if encrypted_required
                && (artifact.checksum_algorithm.is_none() || artifact.checksum.is_none())
            {
                return Err(TerminalPersistenceV2Error::InvalidData(
                    "encrypted export artifact must include a stored-bytes checksum".to_string(),
                ));
            }

            let artifact_id = artifact.id.clone();
            let artifact_kind = artifact.artifact_kind.clone();
            let encryption_state = artifact.encryption_state.clone();
            let checksum_algorithm = artifact.checksum_algorithm.clone();
            let checksum = artifact.checksum.clone();
            let verification = serde_json::json!({
                "artifact_id": artifact_id,
                "artifact_ref_hash": artifact_ref_hash.clone(),
                "artifact_ref_stored": false,
                "artifact_kind": artifact_kind,
                "artifact_state": "verified",
                "encryption_state": encryption_state,
                "key_ref_present": artifact.key_ref.is_some(),
                "checksum_algorithm": checksum_algorithm,
                "checksum": checksum,
                "size_bytes": artifact.size_bytes,
                "verified_at_ms": now,
                "raw_export": raw_export,
                "encrypted_required": encrypted_required,
                "metadata": input.metadata,
            });
            let manifest_json = merge_json_field(
                request.manifest_json.as_deref(),
                "artifact_verification",
                verification.clone(),
            )?;

            diesel::update(
                terminal_external_artifacts::table
                    .filter(terminal_external_artifacts::id.eq(&artifact.id)),
            )
            .set((
                terminal_external_artifacts::state.eq("verified"),
                terminal_external_artifacts::verified_at_ms.eq(Some(now)),
            ))
            .execute(connection)?;

            diesel::update(
                terminal_export_requests::table
                    .filter(terminal_export_requests::id.eq(&input.export_request_id)),
            )
            .set((
                terminal_export_requests::state.eq("succeeded"),
                terminal_export_requests::completed_at_ms.eq(Some(now)),
                terminal_export_requests::manifest_json.eq(manifest_json),
                terminal_export_requests::error.eq(Option::<String>::None),
            ))
            .execute(connection)?;

            Ok(ExportArtifactVerificationRecord {
                export_request_id: input.export_request_id,
                artifact_id: artifact.id,
                artifact_ref_hash,
                export_state: "succeeded".to_string(),
                artifact_state: "verified".to_string(),
                encryption_state: artifact.encryption_state,
                raw_export,
                encrypted_required,
                verified_at_ms: now,
                checksum_algorithm: artifact.checksum_algorithm,
                checksum: artifact.checksum,
                manifest_json: verification,
            })
        })
    }
}
