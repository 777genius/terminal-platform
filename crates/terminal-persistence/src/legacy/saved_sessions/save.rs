use rusqlite::params;

use super::super::{PersistenceError, SavedNativeSession, SqliteSessionStore};

impl SqliteSessionStore {
    pub fn save_native_session(
        &self,
        session: &SavedNativeSession,
    ) -> Result<(), PersistenceError> {
        let mut connection = self.open_connection()?;
        let transaction = connection.transaction()?;
        transaction.execute(
            "
            INSERT INTO native_saved_sessions (
                session_id,
                route_json,
                title,
                launch_json,
                manifest_json,
                topology_json,
                screens_json,
                saved_at_ms
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(session_id) DO UPDATE SET
                route_json = excluded.route_json,
                title = excluded.title,
                launch_json = excluded.launch_json,
                manifest_json = excluded.manifest_json,
                topology_json = excluded.topology_json,
                screens_json = excluded.screens_json,
                saved_at_ms = excluded.saved_at_ms
            ",
            params![
                session.session_id.0.to_string(),
                serde_json::to_string(&session.route)?,
                session.title.as_deref(),
                serde_json::to_string(&session.launch)?,
                serde_json::to_string(&session.manifest)?,
                serde_json::to_string(&session.topology)?,
                serde_json::to_string(&session.screens)?,
                session.saved_at_ms,
            ],
        )?;
        transaction.commit()?;
        Ok(())
    }
}
