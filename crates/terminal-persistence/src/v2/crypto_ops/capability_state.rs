use super::super::*;

impl TerminalPersistenceV2 {
    pub fn encryption_capability_state(
        &self,
    ) -> Result<EncryptionCapabilityRecord, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        encryption_capability_state_for_connection(&mut connection, &self.config)
    }
}
