use rusqlite::params;

use super::super::{PersistenceError, SqliteSessionStore};

impl SqliteSessionStore {
    pub fn delete_native_session(
        &self,
        session_id: terminal_domain::SessionId,
    ) -> Result<bool, PersistenceError> {
        let connection = self.open_connection()?;
        let deleted = connection.execute(
            "
            DELETE FROM native_saved_sessions
            WHERE session_id = ?1
            ",
            params![session_id.0.to_string()],
        )?;

        Ok(deleted > 0)
    }
}
