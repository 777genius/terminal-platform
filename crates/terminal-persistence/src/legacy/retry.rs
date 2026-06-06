use std::{thread, time::Duration};

use crate::v2::TerminalPersistenceV2Error;

use super::PersistenceError;

pub(super) fn retry_persistence_operation<T>(
    mut operation: impl FnMut() -> Result<T, PersistenceError>,
) -> Result<T, PersistenceError> {
    let mut last_error = None;
    for attempt in 0..80 {
        match operation() {
            Ok(value) => return Ok(value),
            Err(error) if is_retryable_persistence_error(&error) => {
                last_error = Some(error);
                let backoff_ms = 10 + i64::from(attempt.min(20)) * 5;
                thread::sleep(Duration::from_millis(backoff_ms as u64));
            }
            Err(error) => return Err(error),
        }
    }

    Err(last_error.unwrap_or_else(|| {
        PersistenceError::InvalidData("sqlite operation retry exhausted".to_string())
    }))
}

fn is_retryable_persistence_error(error: &PersistenceError) -> bool {
    let text = error.to_string().to_ascii_lowercase();
    text.contains("database is locked")
        || text.contains("database table is locked")
        || text.contains("database busy")
        || text.contains("locking protocol")
}

pub(super) fn retry_v2_write<T>(
    mut operation: impl FnMut() -> Result<T, TerminalPersistenceV2Error>,
) -> Result<T, TerminalPersistenceV2Error> {
    let mut last_error = None;
    for attempt in 0..80 {
        match operation() {
            Ok(value) => return Ok(value),
            Err(error) if is_retryable_v2_write_error(&error) => {
                last_error = Some(error);
                let backoff_ms = 10 + i64::from(attempt.min(20)) * 5;
                thread::sleep(Duration::from_millis(backoff_ms as u64));
            }
            Err(error) => return Err(error),
        }
    }

    Err(last_error.unwrap_or_else(|| {
        TerminalPersistenceV2Error::InvalidData("v2 write retry exhausted".to_string())
    }))
}

fn is_retryable_v2_write_error(error: &TerminalPersistenceV2Error) -> bool {
    if matches!(
        error,
        TerminalPersistenceV2Error::WriterAlreadyActive | TerminalPersistenceV2Error::Connection(_)
    ) {
        return true;
    }

    let text = error.to_string().to_ascii_lowercase();
    text.contains("database is locked")
        || text.contains("database table is locked")
        || text.contains("database busy")
        || text.contains("locking protocol")
        || text.contains("active terminal writer generation")
}
