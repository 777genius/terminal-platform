use super::super::super::super::*;

pub(in crate::v2) fn validate_external_artifact_domain(
    artifact_kind: &str,
    state: Option<&str>,
    encryption_state: Option<&str>,
) -> Result<(), TerminalPersistenceV2Error> {
    const ARTIFACT_KINDS: &[&str] =
        &["backup_file", "large_segment", "export_file", "support_bundle", "future_external_store"];
    const STATES: &[&str] =
        &["planned", "available", "verified", "missing", "deleted", "quarantined"];
    const ENCRYPTION_STATES: &[&str] = &["plaintext", "encrypted", "redacted", "crypto_erased"];
    if !ARTIFACT_KINDS.contains(&artifact_kind) {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "unknown external artifact kind: {artifact_kind}"
        )));
    }
    if let Some(state) = state
        && !STATES.contains(&state)
    {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "unknown external artifact state: {state}"
        )));
    }
    if let Some(encryption_state) = encryption_state
        && !ENCRYPTION_STATES.contains(&encryption_state)
    {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "unknown external artifact encryption state: {encryption_state}"
        )));
    }
    Ok(())
}

pub(in crate::v2) fn validate_external_artifact_ref(
    artifact_ref: &str,
) -> Result<(), TerminalPersistenceV2Error> {
    if artifact_ref.trim().is_empty() {
        return Err(TerminalPersistenceV2Error::InvalidData(
            "external artifact ref must not be empty".to_string(),
        ));
    }
    if artifact_ref.len() > 2_048 {
        return Err(TerminalPersistenceV2Error::InvalidData(
            "external artifact ref is too long".to_string(),
        ));
    }
    if artifact_ref.contains('\n') || artifact_ref.contains('\r') {
        return Err(TerminalPersistenceV2Error::InvalidData(
            "external artifact ref must be single-line before hashing".to_string(),
        ));
    }
    Ok(())
}
