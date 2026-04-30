use super::*;

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

    pub fn upsert_redacted_search_document(
        &self,
        input: SearchDocumentInput,
    ) -> Result<SearchDocumentRecord, TerminalPersistenceV2Error> {
        validate_optional_range(input.event_seq_low, input.event_seq_high, "search event")?;
        validate_optional_half_open_range(input.byte_low, input.byte_high, "search byte")?;

        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        let redacted = redact_terminal_text(&input.raw_text);
        let redaction_state =
            if redacted == input.raw_text { "clean".to_string() } else { "redacted".to_string() };
        let source_hash = blake3_hash_text(&input.raw_text);
        let document_id = input.document_id.unwrap_or_else(|| {
            stable_search_document_id(
                &input.session_id,
                input.pane_id.as_deref(),
                input.command_block_id.as_deref(),
                &source_hash,
            )
        });
        let row = NewSearchDocumentRow {
            document_id: document_id.clone(),
            session_id: input.session_id,
            pane_id: input.pane_id,
            command_block_id: input.command_block_id,
            document_kind: input.document_kind.unwrap_or_else(|| "redacted_snippet".to_string()),
            event_seq_low: input.event_seq_low,
            event_seq_high: input.event_seq_high,
            byte_low: input.byte_low,
            byte_high: input.byte_high,
            redaction_profile_id: input
                .redaction_profile_id
                .or_else(|| Some("default".to_string())),
            redaction_state,
            source_hash_algorithm: "blake3".to_string(),
            source_hash,
            text_preview: limit_text_preview(&redacted, 2_048),
            updated_at_ms: now,
            metadata_json: json_metadata(&input.metadata)?,
        };
        insert_into(terminal_search_documents::table)
            .values(&row)
            .on_conflict(terminal_search_documents::document_id)
            .do_update()
            .set((
                terminal_search_documents::session_id.eq(row.session_id.clone()),
                terminal_search_documents::pane_id.eq(row.pane_id.clone()),
                terminal_search_documents::command_block_id.eq(row.command_block_id.clone()),
                terminal_search_documents::document_kind.eq(row.document_kind.clone()),
                terminal_search_documents::event_seq_low.eq(row.event_seq_low),
                terminal_search_documents::event_seq_high.eq(row.event_seq_high),
                terminal_search_documents::byte_low.eq(row.byte_low),
                terminal_search_documents::byte_high.eq(row.byte_high),
                terminal_search_documents::redaction_profile_id
                    .eq(row.redaction_profile_id.clone()),
                terminal_search_documents::redaction_state.eq(row.redaction_state.clone()),
                terminal_search_documents::source_hash_algorithm
                    .eq(row.source_hash_algorithm.clone()),
                terminal_search_documents::source_hash.eq(row.source_hash.clone()),
                terminal_search_documents::text_preview.eq(row.text_preview.clone()),
                terminal_search_documents::updated_at_ms.eq(row.updated_at_ms),
                terminal_search_documents::metadata_json.eq(row.metadata_json.clone()),
            ))
            .execute(&mut connection)?;

        terminal_search_documents::table
            .filter(terminal_search_documents::document_id.eq(document_id))
            .select(SearchDocumentRow::as_select())
            .first::<SearchDocumentRow>(&mut connection)?
            .try_into()
    }

    pub fn list_search_documents(
        &self,
        session_id: &str,
        limit: i64,
    ) -> Result<Vec<SearchDocumentRecord>, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        terminal_search_documents::table
            .filter(terminal_search_documents::session_id.eq(session_id))
            .order(terminal_search_documents::updated_at_ms.desc())
            .limit(limit.max(1))
            .select(SearchDocumentRow::as_select())
            .load::<SearchDocumentRow>(&mut connection)?
            .into_iter()
            .map(TryInto::try_into)
            .collect()
    }

    pub fn create_ai_context_package(
        &self,
        input: AiContextPackageInput,
    ) -> Result<AiContextPackageRecord, TerminalPersistenceV2Error> {
        if input.include_raw {
            return Err(TerminalPersistenceV2Error::InvalidData(
                "AI context packages cannot include raw transcript by default".to_string(),
            ));
        }
        let item_limit = input.max_items.unwrap_or(32).clamp(1, 256);
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        connection.transaction::<_, TerminalPersistenceV2Error, _>(|connection| {
            let id = input.id.unwrap_or_else(new_id);
            let row = NewAiContextPackageRow {
                id: id.clone(),
                session_id: input.session_id.clone(),
                pane_id: input.pane_id.clone(),
                state: "ready".to_string(),
                redaction_profile_id: input
                    .redaction_profile_id
                    .or_else(|| Some("default".to_string())),
                include_raw: 0,
                requested_at_ms: now,
                built_at_ms: Some(now),
                item_count: 0,
                manifest_json: None,
                metadata_json: json_metadata(&input.metadata)?,
            };
            insert_into(terminal_ai_context_packages::table)
                .values(&row)
                .execute(connection)?;

            let mut inserted_items = Vec::new();
            inserted_items.extend(insert_ai_context_items_from_command_history(
                connection,
                &id,
                input.session_id.as_deref(),
                input.pane_id.as_deref(),
                item_limit / 2,
            )?);
            let remaining = item_limit.saturating_sub(i64::try_from(inserted_items.len()).unwrap_or(0));
            if remaining > 0 {
                inserted_items.extend(insert_ai_context_items_from_search_documents(
                    connection,
                    &id,
                    input.session_id.as_deref(),
                    input.pane_id.as_deref(),
                    remaining,
                )?);
            }

            let findings = insert_prompt_injection_findings_for_items(connection, &id, &inserted_items, now)?;
            let manifest = serde_json::json!({
                "kind": "ai_context",
                "session_id": input.session_id,
                "pane_id": input.pane_id,
                "include_raw": false,
                "raw_terminal_output": false,
                "raw_command_text": false,
                "raw_content_included": false,
                "data_only": true,
                "prompt_injection_text_is_data": true,
                "action_approval_required": true,
                "item_count": inserted_items.len(),
                "prompt_injection_finding_count": findings,
                "redaction_profile_id": row.redaction_profile_id,
                "included_classes": ["class_public_diagnostic", "class_local_metadata", "class_user_context_redacted"],
                "excluded_classes": ["class_sensitive_content", "class_secret_material"],
            });
            diesel::update(
                terminal_ai_context_packages::table.filter(terminal_ai_context_packages::id.eq(&id)),
            )
            .set((
                terminal_ai_context_packages::item_count.eq(i64::try_from(inserted_items.len()).unwrap_or(i64::MAX)),
                terminal_ai_context_packages::manifest_json.eq(Some(serde_json::to_string(&manifest)?)),
            ))
            .execute(connection)?;

            AiContextPackageRecord::try_from(load_ai_context_package(connection, &id)?)
        })
    }

    pub fn list_ai_context_items(
        &self,
        package_id: &str,
    ) -> Result<Vec<AiContextItemRecord>, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        terminal_ai_context_items::table
            .filter(terminal_ai_context_items::package_id.eq(package_id))
            .order(terminal_ai_context_items::source_kind.asc())
            .select(AiContextItemRow::as_select())
            .load::<AiContextItemRow>(&mut connection)?
            .into_iter()
            .map(TryInto::try_into)
            .collect()
    }

    pub fn list_prompt_injection_findings(
        &self,
        package_id: &str,
    ) -> Result<Vec<PromptInjectionFindingRecord>, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        terminal_prompt_injection_findings::table
            .filter(terminal_prompt_injection_findings::package_id.eq(Some(package_id.to_string())))
            .order(terminal_prompt_injection_findings::detected_at_ms.desc())
            .select(PromptInjectionFindingRow::as_select())
            .load::<PromptInjectionFindingRow>(&mut connection)?
            .into_iter()
            .map(TryInto::try_into)
            .collect()
    }

    pub fn request_ai_action_approval(
        &self,
        input: AiActionApprovalInput,
    ) -> Result<AiActionApprovalRecord, TerminalPersistenceV2Error> {
        validate_ai_action_kind(&input.action_kind)?;
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        let row = NewAiActionApprovalRow {
            id: input.id.unwrap_or_else(new_id),
            package_id: Some(input.package_id),
            action_kind: input.action_kind,
            state: "pending".to_string(),
            requester_ref_hash: input.requester_ref.map(|value| blake3_hash_text(&value)),
            approver_ref_hash: None,
            requested_at_ms: now,
            decided_at_ms: None,
            expires_at_ms: input.expires_at_ms,
            metadata_json: json_metadata(&input.metadata)?,
        };
        insert_into(terminal_ai_action_approvals::table).values(&row).execute(&mut connection)?;
        Ok(AiActionApprovalRecord::try_from(row)?)
    }

    pub fn decide_ai_action_approval(
        &self,
        input: AiActionDecisionInput,
    ) -> Result<AiActionApprovalRecord, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        let existing = terminal_ai_action_approvals::table
            .filter(terminal_ai_action_approvals::id.eq(&input.approval_id))
            .select(AiActionApprovalRow::as_select())
            .first::<AiActionApprovalRow>(&mut connection)?;
        if existing.state != "pending" {
            return Err(TerminalPersistenceV2Error::InvalidData(format!(
                "AI action approval cannot be decided from state {}",
                existing.state
            )));
        }
        let state = if input.approved { "approved" } else { "denied" };
        let metadata_json = merge_json_field(
            existing.metadata_json.as_deref(),
            "decision",
            serde_json::json!({
                "approved": input.approved,
                "decided_at_ms": now,
                "metadata": input.metadata,
            }),
        )?;
        diesel::update(
            terminal_ai_action_approvals::table
                .filter(terminal_ai_action_approvals::id.eq(&input.approval_id)),
        )
        .set((
            terminal_ai_action_approvals::state.eq(state),
            terminal_ai_action_approvals::approver_ref_hash
                .eq(input.approver_ref.map(|value| blake3_hash_text(&value))),
            terminal_ai_action_approvals::decided_at_ms.eq(Some(now)),
            terminal_ai_action_approvals::metadata_json.eq(metadata_json),
        ))
        .execute(&mut connection)?;
        AiActionApprovalRecord::try_from(load_ai_action_approval(
            &mut connection,
            &input.approval_id,
        )?)
    }
}
