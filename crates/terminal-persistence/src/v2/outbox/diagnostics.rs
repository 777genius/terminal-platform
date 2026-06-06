use super::super::*;

impl TerminalPersistenceV2 {
    pub fn outbox_diagnostics(
        &self,
    ) -> Result<OutboxDiagnosticsRecord, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        collect_outbox_diagnostics(&mut connection, self.config.clock.now_ms())
    }
}
