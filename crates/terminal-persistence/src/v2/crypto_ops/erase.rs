use super::super::*;

impl TerminalPersistenceV2 {
    pub fn complete_crypto_erase(
        &self,
        input: CryptoEraseInput,
    ) -> Result<CryptoEraseRecord, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        connection.transaction::<_, TerminalPersistenceV2Error, _>(|connection| {
            let key = load_crypto_key_for_erase(connection, &input.key_id)?;
            let key_ref_hash = blake3_hash_text(&key.key_ref);
            mark_crypto_key_destroyed(connection, &key.id, now)?;

            let delete_request = insert_crypto_erase_delete_request(connection, &input, now)?;
            insert_crypto_erase_key_event(
                connection,
                &key,
                &key_ref_hash,
                &delete_request.id,
                now,
            )?;

            let evidence = crypto_erase_evidence(&key, &key_ref_hash);
            let tombstone = insert_crypto_erase_tombstone(
                connection,
                &input,
                &delete_request.id,
                &evidence,
                now,
            )?;

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
}

fn load_crypto_key_for_erase(
    connection: &mut SqliteConnection,
    key_id: &str,
) -> Result<CryptoKeyRow, TerminalPersistenceV2Error> {
    terminal_crypto_keys::table
        .filter(terminal_crypto_keys::id.eq(key_id))
        .select(CryptoKeyRow::as_select())
        .first::<CryptoKeyRow>(connection)
        .map_err(Into::into)
}

fn mark_crypto_key_destroyed(
    connection: &mut SqliteConnection,
    key_id: &str,
    now: i64,
) -> Result<(), TerminalPersistenceV2Error> {
    diesel::update(terminal_crypto_keys::table.filter(terminal_crypto_keys::id.eq(key_id)))
        .set((
            terminal_crypto_keys::state.eq("destroyed"),
            terminal_crypto_keys::destroyed_at_ms.eq(Some(now)),
        ))
        .execute(connection)?;
    Ok(())
}

fn insert_crypto_erase_delete_request(
    connection: &mut SqliteConnection,
    input: &CryptoEraseInput,
    now: i64,
) -> Result<NewDeleteRequestRow, TerminalPersistenceV2Error> {
    let delete_request = NewDeleteRequestRow {
        id: input.id.clone().unwrap_or_else(new_id),
        session_id: input.session_id.clone(),
        request_kind: "crypto_erase".to_string(),
        state: "completed".to_string(),
        policy_id: None,
        requested_at_ms: now,
        approved_at_ms: Some(now),
        completed_at_ms: Some(now),
        requester_ref_hash: input.requester_ref.as_ref().map(|value| blake3_hash_text(value)),
        reason: input.reason.clone(),
        metadata_json: json_metadata(&input.metadata)?,
    };
    insert_into(terminal_delete_requests::table).values(&delete_request).execute(connection)?;
    Ok(delete_request)
}

fn insert_crypto_erase_key_event(
    connection: &mut SqliteConnection,
    key: &CryptoKeyRow,
    key_ref_hash: &str,
    delete_request_id: &str,
    now: i64,
) -> Result<(), TerminalPersistenceV2Error> {
    let event = NewCryptoKeyEventRow {
        id: new_id(),
        key_id: Some(key.id.clone()),
        event_kind: "destroyed".to_string(),
        actor: "crypto_erase".to_string(),
        occurred_at_ms: now,
        status: "succeeded".to_string(),
        error_json: None,
        metadata_json: Some(serde_json::to_string(&serde_json::json!({
            "delete_request_id": delete_request_id,
            "key_ref_hash": key_ref_hash,
            "key_material_exported": false
        }))?),
    };
    insert_into(terminal_crypto_key_events::table).values(&event).execute(connection)?;
    Ok(())
}

fn crypto_erase_evidence(key: &CryptoKeyRow, key_ref_hash: &str) -> serde_json::Value {
    serde_json::json!({
        "key_id": key.id,
        "key_kind": key.key_kind,
        "key_ref_hash": key_ref_hash,
        "secure_deletion_limitation": "sqlite_pages_may_retain_old_plaintext_until_vacuum_or_storage_reuse",
        "canonical_history_deleted": false,
        "key_material_exported": false
    })
}

fn insert_crypto_erase_tombstone(
    connection: &mut SqliteConnection,
    input: &CryptoEraseInput,
    delete_request_id: &str,
    evidence: &serde_json::Value,
    now: i64,
) -> Result<NewDeletionTombstoneRow, TerminalPersistenceV2Error> {
    let tombstone = NewDeletionTombstoneRow {
        id: new_id(),
        delete_request_id: Some(delete_request_id.to_string()),
        session_id: input.session_id.clone(),
        deleted_scope: "crypto_key".to_string(),
        policy_id: None,
        deleted_at_ms: now,
        evidence_json: Some(serde_json::to_string(evidence)?),
        metadata_json: None,
    };
    insert_into(terminal_deletion_tombstones::table).values(&tombstone).execute(connection)?;
    Ok(tombstone)
}
