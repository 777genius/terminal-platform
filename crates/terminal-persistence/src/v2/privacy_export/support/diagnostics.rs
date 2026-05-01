use super::super::super::*;

impl TerminalPersistenceV2 {
    pub fn support_bundle_diagnostics(
        &self,
        support_bundle_id: &str,
    ) -> Result<SupportBundleDiagnosticsRecord, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let bundle = load_support_bundle(&mut connection, support_bundle_id)?;
        build_support_bundle_diagnostics(
            &mut connection,
            &self.path,
            &self.config,
            &bundle,
            self.config.clock.now_ms(),
        )
    }
}
