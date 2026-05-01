use super::super::super::*;
use super::encryption::encryption_capability_state_for_connection;

pub(in crate::v2) fn load_feature_gate_state(
    connection: &mut SqliteConnection,
    name: FeatureGateName,
) -> Result<FeatureGateState, TerminalPersistenceV2Error> {
    let row = terminal_feature_gates::table
        .filter(terminal_feature_gates::feature_name.eq(name.as_str()))
        .select(FeatureGateRow::as_select())
        .first::<FeatureGateRow>(connection)?;
    FeatureGateState::parse(&row.state)
}

pub(in crate::v2) fn validate_feature_gate_transition(
    connection: &mut SqliteConnection,
    config: &TerminalPersistenceV2Config,
    name: FeatureGateName,
    state: FeatureGateState,
) -> Result<(), TerminalPersistenceV2Error> {
    if name == FeatureGateName::TerminalPersistenceV2AuthoritativeReads
        && state == FeatureGateState::Enabled
        && load_feature_gate_state(connection, FeatureGateName::TerminalPersistenceV2Authoritative)?
            != FeatureGateState::Enabled
    {
        return Err(TerminalPersistenceV2Error::InvalidData(
            "terminal_persistence_v2_authoritative_reads requires terminal_persistence_v2_authoritative=enabled".to_string(),
        ));
    }

    if name == FeatureGateName::TerminalPersistenceV2Authoritative
        && matches!(state, FeatureGateState::Disabled | FeatureGateState::ForceDisabled)
        && load_feature_gate_state(
            connection,
            FeatureGateName::TerminalPersistenceV2AuthoritativeReads,
        )? == FeatureGateState::Enabled
    {
        return Err(TerminalPersistenceV2Error::InvalidData(
            "disable terminal_persistence_v2_authoritative_reads first before disabling terminal_persistence_v2_authoritative".to_string(),
        ));
    }

    if name == FeatureGateName::EncryptedTerminalHistory
        && state == FeatureGateState::Enabled
        && !encryption_capability_state_for_connection(connection, config)?
            .can_enable_encrypted_history
    {
        return Err(TerminalPersistenceV2Error::InvalidData(
            "encrypted_terminal_history requires an active non-test database key".to_string(),
        ));
    }

    Ok(())
}
