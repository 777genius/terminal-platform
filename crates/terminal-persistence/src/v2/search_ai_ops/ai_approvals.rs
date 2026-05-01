use super::super::*;

impl TerminalPersistenceV2 {
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
