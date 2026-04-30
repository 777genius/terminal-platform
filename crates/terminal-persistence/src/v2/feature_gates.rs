use super::*;

impl TerminalPersistenceV2 {
    pub fn feature_gate_state(
        &self,
        name: FeatureGateName,
    ) -> Result<FeatureGateState, TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        load_feature_gate_state(&mut connection, name)
    }

    pub fn set_feature_gate_state(
        &self,
        name: FeatureGateName,
        state: FeatureGateState,
        reason: Option<&str>,
    ) -> Result<(), TerminalPersistenceV2Error> {
        let mut connection = self.connection()?;
        let now = self.config.clock.now_ms();
        let enabled_at =
            matches!(state, FeatureGateState::Enabled | FeatureGateState::Shadow).then_some(now);
        let disabled_at =
            matches!(state, FeatureGateState::Disabled | FeatureGateState::ForceDisabled)
                .then_some(now);

        connection.transaction::<_, TerminalPersistenceV2Error, _>(|connection| {
            validate_feature_gate_transition(connection, &self.config, name, state)?;
            diesel::update(
                terminal_feature_gates::table
                    .filter(terminal_feature_gates::feature_name.eq(name.as_str())),
            )
            .set((
                terminal_feature_gates::state.eq(state.as_str()),
                terminal_feature_gates::reason.eq(reason.map(ToOwned::to_owned)),
                terminal_feature_gates::enabled_at_ms.eq(enabled_at),
                terminal_feature_gates::disabled_at_ms.eq(disabled_at),
                terminal_feature_gates::updated_at_ms.eq(now),
            ))
            .execute(connection)?;
            Ok(())
        })
    }
}
