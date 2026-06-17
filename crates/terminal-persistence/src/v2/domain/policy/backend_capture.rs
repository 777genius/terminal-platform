use super::super::super::*;

pub(in crate::v2) fn validate_capture_semantics_domain(
    value: &str,
) -> Result<(), TerminalPersistenceV2Error> {
    const CAPTURE_SEMANTICS: &[&str] = &[
        "raw_vt_stream",
        "rendered_ansi_stream",
        "rendered_screen_snapshot",
        "rendered_plaintext_snapshot",
        "mux_structured_surface",
        "imported_text",
        "ui_input",
    ];
    if !CAPTURE_SEMANTICS.contains(&value) {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "unknown capture semantics: {value}"
        )));
    }
    Ok(())
}

pub(in crate::v2) fn validate_capture_strategy_domain(
    value: &str,
) -> Result<(), TerminalPersistenceV2Error> {
    const CAPTURE_STRATEGIES: &[&str] = &[
        "raw_stream",
        "rendered_stream",
        "rendered_snapshot",
        "mux_structured",
        "imported_snapshot",
        "ui_input",
        "unknown",
    ];
    if !CAPTURE_STRATEGIES.contains(&value) {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "unknown capture strategy: {value}"
        )));
    }
    Ok(())
}

pub(in crate::v2) fn validate_command_boundary_confidence_domain(
    value: &str,
) -> Result<(), TerminalPersistenceV2Error> {
    const CONFIDENCE_LEVELS: &[&str] = &["verified", "high", "medium", "low", "none", "unknown"];
    if !CONFIDENCE_LEVELS.contains(&value) {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "unknown command boundary confidence: {value}"
        )));
    }
    Ok(())
}

pub(in crate::v2) fn validate_backend_probe_status_domain(
    value: &str,
) -> Result<(), TerminalPersistenceV2Error> {
    const PROBE_STATUSES: &[&str] = &["passed", "failed", "partial", "stale"];
    if !PROBE_STATUSES.contains(&value) {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "unknown backend probe status: {value}"
        )));
    }
    Ok(())
}

pub(in crate::v2) fn validate_backend_capability_stale_reason(
    value: &str,
) -> Result<(), TerminalPersistenceV2Error> {
    const REASONS: &[&str] = &[
        "backend_version_changed",
        "backend_binary_path_changed",
        "backend_config_changed",
        "probe_failed",
        "manual_invalidation",
    ];
    if !REASONS.contains(&value) {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "unknown backend capability stale reason: {value}"
        )));
    }
    Ok(())
}
