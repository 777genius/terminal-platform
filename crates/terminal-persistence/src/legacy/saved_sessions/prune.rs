use rusqlite::params;

use super::super::{PersistenceError, PrunedSavedSessions, SqliteSessionStore};

impl SqliteSessionStore {
    pub fn prune_native_sessions(
        &self,
        keep_latest: usize,
    ) -> Result<PrunedSavedSessions, PersistenceError> {
        let mut connection = self.open_connection()?;
        let transaction = connection.transaction()?;
        let deleted_count = transaction.execute(
            "
            DELETE FROM native_saved_sessions
            WHERE session_id IN (
                SELECT session_id
                FROM native_saved_sessions
                ORDER BY saved_at_ms DESC, session_id DESC
                LIMIT -1 OFFSET ?1
            )
            ",
            params![keep_latest as i64],
        )?;
        let kept_count =
            transaction.query_row("SELECT COUNT(*) FROM native_saved_sessions", [], |row| {
                row.get::<_, i64>(0)
            })? as usize;
        transaction.commit()?;

        Ok(PrunedSavedSessions { deleted_count, kept_count })
    }
}
