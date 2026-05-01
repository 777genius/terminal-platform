use super::super::super::*;
use super::super::*;

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

pub(in crate::v2) fn validate_external_artifact_target_ref(
    artifact_ref: &str,
    source_db_path: &Path,
) -> Result<(), TerminalPersistenceV2Error> {
    let Some(target_path) = path_like_artifact_ref(artifact_ref) else {
        return Ok(());
    };
    let source_canonical = source_db_path.canonicalize()?;
    let Some(target_normalized) = normalize_artifact_target_path(&target_path) else {
        return Ok(());
    };
    let forbidden_targets = [
        source_canonical.clone(),
        sqlite_sidecar_path(&source_canonical, "-wal"),
        sqlite_sidecar_path(&source_canonical, "-shm"),
    ];
    if forbidden_targets
        .iter()
        .any(|forbidden| paths_equal_for_platform(forbidden, &target_normalized))
    {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "external artifact ref cannot point at the live database or SQLite sidecar: {}",
            target_normalized.display()
        )));
    }
    Ok(())
}

pub(in crate::v2) fn path_like_artifact_ref(artifact_ref: &str) -> Option<PathBuf> {
    let trimmed = artifact_ref.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.contains("://") {
        return None;
    }
    let path = PathBuf::from(trimmed);
    if path.is_absolute()
        || trimmed.starts_with('.')
        || trimmed.contains('\\')
        || trimmed.contains('/')
        || looks_like_windows_drive_path(trimmed)
    {
        Some(path)
    } else {
        None
    }
}

pub(in crate::v2) fn looks_like_windows_drive_path(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 3
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
        && bytes[0].is_ascii_alphabetic()
}

pub(in crate::v2) fn normalize_artifact_target_path(path: &Path) -> Option<PathBuf> {
    if let Ok(canonical) = path.canonicalize() {
        return Some(canonical);
    }
    let file_name = path.file_name()?;
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    parent.canonicalize().ok().map(|parent| parent.join(file_name))
}
