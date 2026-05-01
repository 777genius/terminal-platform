use super::super::super::*;

impl TerminalPersistenceV2 {
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
}
