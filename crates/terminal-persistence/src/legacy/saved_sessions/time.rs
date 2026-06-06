use std::time::{SystemTime, UNIX_EPOCH};

use super::super::{PersistenceError, SqliteSessionStore};

impl SqliteSessionStore {
    pub fn save_timestamp_ms() -> Result<i64, PersistenceError> {
        Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as i64)
    }
}
