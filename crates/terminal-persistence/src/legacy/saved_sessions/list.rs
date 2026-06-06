use super::super::{
    PersistenceError, SavedSessionSummary, SqliteSessionStore,
    summary::{SavedSessionSummaryRow, decode_saved_session_summary_row},
};

impl SqliteSessionStore {
    pub fn list_native_sessions(&self) -> Result<Vec<SavedSessionSummary>, PersistenceError> {
        let connection = self.open_connection()?;
        let mut statement = connection.prepare(
            "
            SELECT session_id, route_json, title, launch_json, manifest_json, topology_json, saved_at_ms
            FROM native_saved_sessions
            ORDER BY saved_at_ms DESC
            ",
        )?;
        let rows = statement.query_map([], |row| {
            Ok::<SavedSessionSummaryRow, rusqlite::Error>((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, i64>(6)?,
            ))
        })?;

        let mut sessions = Vec::new();
        for row in rows {
            let row = row?;
            if let Ok(session) = decode_saved_session_summary_row(row) {
                sessions.push(session);
            }
        }

        Ok(sessions)
    }
}
