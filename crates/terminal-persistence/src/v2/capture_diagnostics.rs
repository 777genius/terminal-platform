use super::*;

impl TerminalPersistenceV2 {
    pub fn compression_diagnostics(
        &self,
    ) -> Result<CompressionDiagnosticsRecord, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        collect_compression_diagnostics(&mut connection, self.config.clock.now_ms())
    }

    pub fn retention_diagnostics(
        &self,
        selected_policy_id: Option<&str>,
    ) -> Result<RetentionDiagnosticsRecord, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        collect_retention_diagnostics(
            &mut connection,
            self.config.clock.now_ms(),
            selected_policy_id,
        )
    }
}
