use super::super::*;

impl TerminalPersistenceV2 {
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
                redaction_profile_id: input.redaction_profile_id.or_else(|| Some("default".to_string())),
                include_raw: 0,
                requested_at_ms: now,
                built_at_ms: Some(now),
                item_count: 0,
                manifest_json: None,
                metadata_json: json_metadata(&input.metadata)?,
            };
            insert_into(terminal_ai_context_packages::table).values(&row).execute(connection)?;

            let mut inserted_items = Vec::new();
            inserted_items.extend(insert_ai_context_items_from_command_history(
                connection,
                &id,
                input.session_id.as_deref(),
                input.pane_id.as_deref(),
                item_limit / 2,
            )?);
            let remaining =
                item_limit.saturating_sub(i64::try_from(inserted_items.len()).unwrap_or(0));
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
                terminal_ai_context_packages::table
                    .filter(terminal_ai_context_packages::id.eq(&id)),
            )
            .set((
                terminal_ai_context_packages::item_count
                    .eq(i64::try_from(inserted_items.len()).unwrap_or(i64::MAX)),
                terminal_ai_context_packages::manifest_json
                    .eq(Some(serde_json::to_string(&manifest)?)),
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
}
