use super::super::*;

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

    pub fn support_bundle_diagnostics(
        &self,
        support_bundle_id: &str,
    ) -> Result<SupportBundleDiagnosticsRecord, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let bundle = load_support_bundle(&mut connection, support_bundle_id)?;
        build_support_bundle_diagnostics(
            &mut connection,
            &self.path,
            &self.config,
            &bundle,
            self.config.clock.now_ms(),
        )
    }

    pub fn complete_support_bundle(
        &self,
        input: SupportBundleCompletionInput,
    ) -> Result<SupportBundleRecord, TerminalPersistenceV2Error> {
        if let Some(artifact_ref) = input.artifact_ref.as_deref() {
            validate_external_artifact_ref(artifact_ref)?;
            validate_external_artifact_target_ref(artifact_ref, &self.path)?;
        }
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        connection.transaction::<_, TerminalPersistenceV2Error, _>(|connection| {
            let bundle = load_support_bundle(connection, &input.support_bundle_id)?;
            if bundle.state == "succeeded" || bundle.state == "failed" {
                return Err(TerminalPersistenceV2Error::InvalidData(format!(
                    "support bundle cannot be completed from state {}",
                    bundle.state
                )));
            }
            if bundle.include_raw != 0 {
                if load_feature_gate_state(connection, FeatureGateName::RawHistoryExport)?
                    != FeatureGateState::Enabled
                {
                    return Err(TerminalPersistenceV2Error::InvalidData(
                        "raw support bundle completion requires raw history export gate"
                            .to_string(),
                    ));
                }
                ensure_no_open_critical_health_records(connection, None, "raw support bundle")?;
            }

            let artifact_verification = if let Some(artifact_ref) = input.artifact_ref.as_deref() {
                let artifact_ref_hash = blake3_hash_text(artifact_ref);
                if bundle.output_ref_hash.as_deref() != Some(artifact_ref_hash.as_str()) {
                    return Err(TerminalPersistenceV2Error::InvalidData(
                        "support bundle artifact ref does not match request output_ref hash"
                            .to_string(),
                    ));
                }
                let artifact = terminal_external_artifacts::table
                    .filter(terminal_external_artifacts::artifact_ref_hash.eq(&artifact_ref_hash))
                    .select(ExternalArtifactRow::as_select())
                    .first::<ExternalArtifactRow>(connection)?;
                if artifact.artifact_kind != "support_bundle" {
                    return Err(TerminalPersistenceV2Error::InvalidData(format!(
                        "support bundle completion requires support_bundle artifact, got {}",
                        artifact.artifact_kind
                    )));
                }
                if artifact.state != "available" && artifact.state != "verified" {
                    return Err(TerminalPersistenceV2Error::InvalidData(format!(
                        "support bundle artifact must be available or verified, got {}",
                        artifact.state
                    )));
                }
                if bundle.include_raw != 0 && artifact.encryption_state != "encrypted" {
                    return Err(TerminalPersistenceV2Error::InvalidData(
                        "raw support bundle must complete into an encrypted artifact".to_string(),
                    ));
                }
                if artifact.encryption_state == "encrypted"
                    && (artifact.key_ref.is_none()
                        || artifact.checksum_algorithm.is_none()
                        || artifact.checksum.is_none())
                {
                    return Err(TerminalPersistenceV2Error::InvalidData(
                        "encrypted support bundle artifact must include key ref and checksum"
                            .to_string(),
                    ));
                }

                diesel::update(
                    terminal_external_artifacts::table
                        .filter(terminal_external_artifacts::id.eq(&artifact.id)),
                )
                .set((
                    terminal_external_artifacts::state.eq("verified"),
                    terminal_external_artifacts::verified_at_ms.eq(Some(now)),
                ))
                .execute(connection)?;

                Some(serde_json::json!({
                    "artifact_id": artifact.id,
                    "artifact_ref_hash": artifact_ref_hash,
                    "artifact_ref_stored": false,
                    "artifact_kind": artifact.artifact_kind,
                    "artifact_state": "verified",
                    "encryption_state": artifact.encryption_state,
                    "key_ref_present": artifact.key_ref.is_some(),
                    "checksum_algorithm": artifact.checksum_algorithm,
                    "checksum": artifact.checksum,
                    "size_bytes": artifact.size_bytes,
                    "verified_at_ms": now,
                }))
            } else {
                None
            };

            let diagnostics = build_support_bundle_diagnostics(
                connection,
                &self.path,
                &self.config,
                &bundle,
                now,
            )?;
            let mut manifest = bundle
                .manifest_json
                .as_deref()
                .map(serde_json::from_str)
                .transpose()?
                .unwrap_or_else(|| {
                    privacy_manifest("support_bundle", bundle.include_raw != 0, None)
                });
            if !manifest.is_object() {
                manifest = serde_json::json!({ "legacy_manifest_value": manifest });
            }
            let manifest_object = manifest.as_object_mut().expect("manifest is object");
            manifest_object.insert("diagnostics".to_string(), diagnostics.manifest_json.clone());
            if let Some(artifact_verification) = artifact_verification {
                manifest_object.insert("artifact_verification".to_string(), artifact_verification);
            }
            manifest_object.insert(
                "completion".to_string(),
                serde_json::json!({
                    "completed_at_ms": now,
                    "metadata": input.metadata,
                    "raw_content_included": bundle.include_raw != 0,
                    "raw_content_included_by_default": false,
                }),
            );

            diesel::update(
                terminal_support_bundles::table
                    .filter(terminal_support_bundles::id.eq(&input.support_bundle_id)),
            )
            .set((
                terminal_support_bundles::state.eq("succeeded"),
                terminal_support_bundles::completed_at_ms.eq(Some(now)),
                terminal_support_bundles::manifest_json.eq(Some(serde_json::to_string(&manifest)?)),
                terminal_support_bundles::error.eq(Option::<String>::None),
            ))
            .execute(connection)?;

            SupportBundleRecord::try_from(load_support_bundle(
                connection,
                &input.support_bundle_id,
            )?)
        })
    }
}
