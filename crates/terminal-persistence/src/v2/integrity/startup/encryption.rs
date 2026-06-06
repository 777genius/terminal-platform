use super::super::super::*;
use super::feature_gates::load_feature_gate_state;

pub(in crate::v2) fn enforce_encryption_startup_policy(
    connection: &mut SqliteConnection,
    config: &TerminalPersistenceV2Config,
) -> Result<(), TerminalPersistenceV2Error> {
    let capability = encryption_capability_state_for_connection(connection, config)?;
    if capability.feature_gate_state == FeatureGateState::Enabled.as_str()
        && !capability.can_enable_encrypted_history
    {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "encrypted terminal history cannot start: {}",
            capability.action_required
        )));
    }
    Ok(())
}

pub(in crate::v2) fn encryption_capability_state_for_connection(
    connection: &mut SqliteConnection,
    config: &TerminalPersistenceV2Config,
) -> Result<EncryptionCapabilityRecord, TerminalPersistenceV2Error> {
    let feature_gate_state =
        load_feature_gate_state(connection, FeatureGateName::EncryptedTerminalHistory)?
            .as_str()
            .to_string();
    let active_database_key_count = terminal_crypto_keys::table
        .filter(terminal_crypto_keys::key_kind.eq("database_key"))
        .filter(terminal_crypto_keys::state.eq("active"))
        .count()
        .get_result::<i64>(connection)?;
    let active_non_test_database_key_count = terminal_crypto_keys::table
        .filter(terminal_crypto_keys::key_kind.eq("database_key"))
        .filter(terminal_crypto_keys::state.eq("active"))
        .filter(terminal_crypto_keys::protection_kind.ne("test_plaintext"))
        .count()
        .get_result::<i64>(connection)?;
    let test_plaintext_database_key_count = terminal_crypto_keys::table
        .filter(terminal_crypto_keys::key_kind.eq("database_key"))
        .filter(terminal_crypto_keys::state.eq("active"))
        .filter(terminal_crypto_keys::protection_kind.eq("test_plaintext"))
        .count()
        .get_result::<i64>(connection)?;
    let unavailable_key_count = terminal_crypto_keys::table
        .filter(terminal_crypto_keys::state.eq("unavailable"))
        .count()
        .get_result::<i64>(connection)?;
    let test_key_allowed =
        config.allow_test_plaintext_crypto_keys && test_plaintext_database_key_count > 0;
    let can_enable_encrypted_history = active_non_test_database_key_count > 0 || test_key_allowed;
    let action_required = if can_enable_encrypted_history {
        "none"
    } else if active_database_key_count == 0 {
        "register_active_database_key"
    } else {
        "replace_test_plaintext_key_with_os_protected_key"
    }
    .to_string();

    Ok(EncryptionCapabilityRecord {
        feature_gate_state,
        active_database_key_count,
        active_non_test_database_key_count,
        test_plaintext_database_key_count,
        unavailable_key_count,
        can_enable_encrypted_history,
        plaintext_fallback_allowed: false,
        key_material_exported: false,
        action_required,
    })
}
