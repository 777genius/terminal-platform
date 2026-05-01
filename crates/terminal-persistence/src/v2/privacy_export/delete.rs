use super::super::*;

impl TerminalPersistenceV2 {
    pub fn create_delete_request(
        &self,
        input: DeleteRequestInput,
    ) -> Result<DeleteRequestRecord, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        let row = NewDeleteRequestRow {
            id: input.id.unwrap_or_else(new_id),
            session_id: input.session_id,
            request_kind: input.request_kind.unwrap_or_else(|| "user_delete".to_string()),
            state: "pending".to_string(),
            policy_id: input.policy_id,
            requested_at_ms: now,
            approved_at_ms: None,
            completed_at_ms: None,
            requester_ref_hash: input.requester_ref.map(|value| blake3_hash_text(&value)),
            reason: input.reason,
            metadata_json: json_metadata(&input.metadata)?,
        };
        insert_into(terminal_delete_requests::table).values(&row).execute(&mut connection)?;
        Ok(DeleteRequestRecord::try_from(row)?)
    }

    pub fn complete_delete_request_with_tombstone(
        &self,
        delete_request_id: &str,
        deleted_scope: &str,
        evidence: Option<Value>,
        metadata: Option<Value>,
    ) -> Result<DeletionTombstoneRecord, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        connection.transaction::<_, TerminalPersistenceV2Error, _>(|connection| {
            let request = terminal_delete_requests::table
                .filter(terminal_delete_requests::id.eq(delete_request_id))
                .select(DeleteRequestRow::as_select())
                .first::<DeleteRequestRow>(connection)?;
            if request.state == "completed" {
                return Err(TerminalPersistenceV2Error::InvalidData(
                    "delete request is already completed".to_string(),
                ));
            }

            diesel::update(
                terminal_delete_requests::table
                    .filter(terminal_delete_requests::id.eq(delete_request_id)),
            )
            .set((
                terminal_delete_requests::state.eq("completed"),
                terminal_delete_requests::completed_at_ms.eq(Some(now)),
            ))
            .execute(connection)?;

            let row = NewDeletionTombstoneRow {
                id: new_id(),
                delete_request_id: Some(delete_request_id.to_string()),
                session_id: request.session_id,
                deleted_scope: deleted_scope.to_string(),
                policy_id: request.policy_id,
                deleted_at_ms: now,
                evidence_json: evidence.as_ref().map(serde_json::to_string).transpose()?,
                metadata_json: json_metadata(&metadata)?,
            };
            insert_into(terminal_deletion_tombstones::table).values(&row).execute(connection)?;
            Ok(DeletionTombstoneRecord::try_from(row)?)
        })
    }
}
