use super::*;

impl TerminalPersistenceV2 {
    pub fn register_crypto_key(
        &self,
        input: CryptoKeyInput,
    ) -> Result<CryptoKeyRecord, TerminalPersistenceV2Error> {
        validate_crypto_key_domain(
            &input.key_kind,
            &input.protection_kind,
            input.state.as_deref(),
        )?;
        validate_crypto_key_ref(&input.key_ref)?;
        if input.protection_kind == "test_plaintext"
            && !self.config.allow_test_plaintext_crypto_keys
        {
            return Err(TerminalPersistenceV2Error::InvalidData(
                "test_plaintext crypto keys are allowed only in test configuration".to_string(),
            ));
        }

        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        let state = input.state.unwrap_or_else(|| "active".to_string());
        let row = NewCryptoKeyRow {
            id: input.id.unwrap_or_else(new_id),
            key_kind: input.key_kind,
            key_ref: input.key_ref,
            protection_kind: input.protection_kind,
            state,
            created_at_ms: now,
            rotated_at_ms: None,
            destroyed_at_ms: None,
            capability_report_json: input
                .capability_report
                .as_ref()
                .map(serde_json::to_string)
                .transpose()?,
            error_json: input.error.as_ref().map(serde_json::to_string).transpose()?,
            metadata_json: json_metadata(&input.metadata)?,
        };
        insert_into(terminal_crypto_keys::table).values(&row).execute(&mut connection)?;
        Ok(CryptoKeyRecord::try_from(row)?)
    }

    pub fn record_crypto_key_event(
        &self,
        input: CryptoKeyEventInput,
    ) -> Result<CryptoKeyEventRecord, TerminalPersistenceV2Error> {
        validate_crypto_key_event_domain(&input.event_kind, &input.status)?;
        let mut connection = self.connection()?;
        let row = NewCryptoKeyEventRow {
            id: input.id.unwrap_or_else(new_id),
            key_id: input.key_id,
            event_kind: input.event_kind,
            actor: input.actor,
            occurred_at_ms: input.occurred_at_ms.unwrap_or_else(|| self.config.clock.now_ms()),
            status: input.status,
            error_json: input.error.as_ref().map(serde_json::to_string).transpose()?,
            metadata_json: json_metadata(&input.metadata)?,
        };
        insert_into(terminal_crypto_key_events::table).values(&row).execute(&mut connection)?;
        Ok(CryptoKeyEventRecord::try_from(row)?)
    }

    pub fn complete_crypto_erase(
        &self,
        input: CryptoEraseInput,
    ) -> Result<CryptoEraseRecord, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        connection.transaction::<_, TerminalPersistenceV2Error, _>(|connection| {
            let key = terminal_crypto_keys::table
                .filter(terminal_crypto_keys::id.eq(&input.key_id))
                .select(CryptoKeyRow::as_select())
                .first::<CryptoKeyRow>(connection)?;
            let key_ref_hash = blake3_hash_text(&key.key_ref);
            diesel::update(terminal_crypto_keys::table.filter(terminal_crypto_keys::id.eq(&key.id)))
                .set((
                    terminal_crypto_keys::state.eq("destroyed"),
                    terminal_crypto_keys::destroyed_at_ms.eq(Some(now)),
                ))
                .execute(connection)?;

            let delete_request = NewDeleteRequestRow {
                id: input.id.unwrap_or_else(new_id),
                session_id: input.session_id.clone(),
                request_kind: "crypto_erase".to_string(),
                state: "completed".to_string(),
                policy_id: None,
                requested_at_ms: now,
                approved_at_ms: Some(now),
                completed_at_ms: Some(now),
                requester_ref_hash: input.requester_ref.map(|value| blake3_hash_text(&value)),
                reason: input.reason,
                metadata_json: json_metadata(&input.metadata)?,
            };
            insert_into(terminal_delete_requests::table)
                .values(&delete_request)
                .execute(connection)?;

            let event = NewCryptoKeyEventRow {
                id: new_id(),
                key_id: Some(key.id.clone()),
                event_kind: "destroyed".to_string(),
                actor: "crypto_erase".to_string(),
                occurred_at_ms: now,
                status: "succeeded".to_string(),
                error_json: None,
                metadata_json: Some(serde_json::to_string(&serde_json::json!({
                    "delete_request_id": delete_request.id,
                    "key_ref_hash": key_ref_hash,
                    "key_material_exported": false
                }))?),
            };
            insert_into(terminal_crypto_key_events::table).values(&event).execute(connection)?;

            let evidence = serde_json::json!({
                "key_id": key.id,
                "key_kind": key.key_kind,
                "key_ref_hash": key_ref_hash,
                "secure_deletion_limitation": "sqlite_pages_may_retain_old_plaintext_until_vacuum_or_storage_reuse",
                "canonical_history_deleted": false,
                "key_material_exported": false
            });
            let tombstone = NewDeletionTombstoneRow {
                id: new_id(),
                delete_request_id: Some(delete_request.id.clone()),
                session_id: input.session_id,
                deleted_scope: "crypto_key".to_string(),
                policy_id: None,
                deleted_at_ms: now,
                evidence_json: Some(serde_json::to_string(&evidence)?),
                metadata_json: None,
            };
            insert_into(terminal_deletion_tombstones::table)
                .values(&tombstone)
                .execute(connection)?;

            Ok(CryptoEraseRecord {
                key_id: key.id,
                key_ref_hash,
                delete_request_id: delete_request.id,
                tombstone_id: tombstone.id,
                state: "completed".to_string(),
                secure_deletion_limitation: evidence["secure_deletion_limitation"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            })
        })
    }

    pub fn encryption_capability_state(
        &self,
    ) -> Result<EncryptionCapabilityRecord, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        encryption_capability_state_for_connection(&mut connection, &self.config)
    }
}
