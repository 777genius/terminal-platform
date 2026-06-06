use super::super::*;

impl TerminalPersistenceV2 {
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
}
