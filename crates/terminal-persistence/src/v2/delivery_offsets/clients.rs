use super::super::*;

impl TerminalPersistenceV2 {
    pub fn upsert_delivery_client(
        &self,
        input: DeliveryClientInput,
    ) -> Result<DeliveryClientRecord, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        let id = input.id.unwrap_or_else(new_id);
        let row = NewDeliveryClientRow {
            id: id.clone(),
            client_kind: input.client_kind,
            install_ref_hash: input.install_ref_hash,
            browser_profile_ref_hash: input.browser_profile_ref_hash,
            user_agent_hash: input.user_agent_hash,
            created_at_ms: now,
            last_seen_at_ms: now,
            trust_state: input.trust_state.unwrap_or_else(|| "local_unverified".to_string()),
        };

        insert_into(terminal_clients::table)
            .values(&row)
            .on_conflict(terminal_clients::id)
            .do_update()
            .set((
                terminal_clients::client_kind.eq(row.client_kind.clone()),
                terminal_clients::install_ref_hash.eq(row.install_ref_hash.clone()),
                terminal_clients::browser_profile_ref_hash.eq(row.browser_profile_ref_hash.clone()),
                terminal_clients::user_agent_hash.eq(row.user_agent_hash.clone()),
                terminal_clients::last_seen_at_ms.eq(row.last_seen_at_ms),
                terminal_clients::trust_state.eq(row.trust_state.clone()),
            ))
            .execute(&mut connection)?;

        Ok(DeliveryClientRecord {
            id,
            client_kind: row.client_kind,
            last_seen_at_ms: now,
            trust_state: row.trust_state,
        })
    }
}
