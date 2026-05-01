use super::super::*;

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
}
