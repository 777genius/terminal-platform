use super::super::super::super::*;

pub(in crate::v2) fn validate_crypto_key_domain(
    key_kind: &str,
    protection_kind: &str,
    state: Option<&str>,
) -> Result<(), TerminalPersistenceV2Error> {
    const KEY_KINDS: &[&str] = &["database_key", "export_key", "artifact_key"];
    const PROTECTION_KINDS: &[&str] = &[
        "windows_credential_manager",
        "dpapi_user",
        "dpapi_machine",
        "macos_keychain",
        "linux_secret_service",
        "test_plaintext",
    ];
    const STATES: &[&str] = &["active", "rotating", "disabled", "destroyed", "unavailable"];
    if !KEY_KINDS.contains(&key_kind) {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "unknown crypto key kind: {key_kind}"
        )));
    }
    if !PROTECTION_KINDS.contains(&protection_kind) {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "unknown crypto key protection kind: {protection_kind}"
        )));
    }
    if let Some(state) = state
        && !STATES.contains(&state)
    {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "unknown crypto key state: {state}"
        )));
    }
    Ok(())
}

pub(in crate::v2) fn validate_crypto_key_event_domain(
    event_kind: &str,
    status: &str,
) -> Result<(), TerminalPersistenceV2Error> {
    const EVENT_KINDS: &[&str] = &[
        "created",
        "unlocked",
        "lock_failed",
        "rotated",
        "destroy_requested",
        "destroyed",
        "recovery_failed",
    ];
    const STATUSES: &[&str] = &["succeeded", "failed", "skipped"];
    if !EVENT_KINDS.contains(&event_kind) {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "unknown crypto key event kind: {event_kind}"
        )));
    }
    if !STATUSES.contains(&status) {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "unknown crypto key event status: {status}"
        )));
    }
    Ok(())
}

pub(in crate::v2) fn validate_crypto_key_ref(
    key_ref: &str,
) -> Result<(), TerminalPersistenceV2Error> {
    if key_ref.trim().is_empty() {
        return Err(TerminalPersistenceV2Error::InvalidData(
            "crypto key_ref must not be empty".to_string(),
        ));
    }
    if key_ref.len() > 512 {
        return Err(TerminalPersistenceV2Error::InvalidData(
            "crypto key_ref must stay an opaque short reference, not key material".to_string(),
        ));
    }
    if key_ref.contains('\n') || key_ref.contains('\r') {
        return Err(TerminalPersistenceV2Error::InvalidData(
            "crypto key_ref must be a single-line opaque reference".to_string(),
        ));
    }
    let lower = key_ref.to_ascii_lowercase();
    if lower.contains("begin ") || lower.contains("private key") || lower.contains("secret key") {
        return Err(TerminalPersistenceV2Error::InvalidData(
            "crypto key_ref must not contain key material".to_string(),
        ));
    }
    Ok(())
}
