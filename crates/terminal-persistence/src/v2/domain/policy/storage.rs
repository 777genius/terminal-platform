use super::super::super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::v2) struct StoragePressureClassification {
    pub(in crate::v2) state: &'static str,
    pub(in crate::v2) action_taken: &'static str,
    pub(in crate::v2) reason: &'static str,
    pub(in crate::v2) db_over_budget: bool,
    pub(in crate::v2) wal_over_budget: bool,
}

pub(in crate::v2) fn classify_storage_pressure(
    db_file_bytes: Option<i64>,
    wal_file_bytes: Option<i64>,
    config: StoragePressureConfig,
) -> StoragePressureClassification {
    let db_over_budget = config.db_warning_bytes > 0
        && db_file_bytes.is_some_and(|value| value >= config.db_warning_bytes);
    let wal_over_budget = config.wal_warning_bytes > 0
        && wal_file_bytes.is_some_and(|value| value >= config.wal_warning_bytes);
    let (state, action_taken, reason) = match (db_over_budget, wal_over_budget) {
        (true, true) => ("warning", "checkpoint_and_warn", "db_and_wal_file_size_over_budget"),
        (false, true) => ("warning", "checkpoint_recommended", "wal_file_size_over_budget"),
        (true, false) => ("warning", "warn_only", "db_file_size_over_budget"),
        (false, false) => ("ok", "none", "manual_probe"),
    };

    StoragePressureClassification { state, action_taken, reason, db_over_budget, wal_over_budget }
}

pub(in crate::v2) fn validate_storage_pressure_domain(
    state: &str,
    action_taken: &str,
) -> Result<(), TerminalPersistenceV2Error> {
    const STATES: &[&str] = &["ok", "warning", "degraded", "full", "unknown"];
    const ACTIONS: &[&str] = &[
        "none",
        "warn_only",
        "checkpoint_recommended",
        "checkpoint_and_warn",
        "degrade_with_gap",
        "fail_closed",
    ];
    if !STATES.contains(&state) {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "unknown storage pressure state: {state}"
        )));
    }
    if !ACTIONS.contains(&action_taken) {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "unknown storage pressure action: {action_taken}"
        )));
    }
    Ok(())
}

pub(in crate::v2) fn is_storage_full_like_error(error: &TerminalPersistenceV2Error) -> bool {
    match error {
        TerminalPersistenceV2Error::Query(DieselError::DatabaseError(_, info)) => {
            let message = info.message().to_ascii_lowercase();
            message.contains("sqlite_full")
                || message.contains("database or disk is full")
                || message.contains("disk is full")
                || message.contains("database is full")
        }
        TerminalPersistenceV2Error::Io(error) => {
            let message = error.to_string().to_ascii_lowercase();
            message.contains("disk full")
                || message.contains("disk is full")
                || message.contains("not enough space")
        }
        _ => false,
    }
}
