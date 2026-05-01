use super::super::*;

impl TerminalPersistenceV2 {
    pub(in crate::v2) fn ensure_raw_history_export_enabled(
        &self,
    ) -> Result<(), TerminalPersistenceV2Error> {
        match self.feature_gate_state(FeatureGateName::RawHistoryExport)? {
            FeatureGateState::Enabled => Ok(()),
            other => Err(TerminalPersistenceV2Error::InvalidData(format!(
                "raw history export is disabled by feature gate: {}",
                other.as_str()
            ))),
        }
    }
}
