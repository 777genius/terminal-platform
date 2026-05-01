use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{OptionalExtension, params};

use super::{
    PersistenceError, PrunedSavedSessions, SavedNativeSession, SavedSessionSummary,
    SqliteSessionStore,
    summary::{SavedSessionSummaryRow, decode_saved_session_summary_row},
};

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
                session.title,
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

    pub fn load_native_session(
        &self,
        session_id: terminal_domain::SessionId,
    ) -> Result<Option<SavedNativeSession>, PersistenceError> {
        let connection = self.open_connection()?;
        let row = connection
            .query_row(
                "
                SELECT route_json, title, launch_json, manifest_json, topology_json, screens_json, saved_at_ms
                FROM native_saved_sessions
                WHERE session_id = ?1
                ",
                params![session_id.0.to_string()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, i64>(6)?,
                    ))
                },
            )
            .optional()?;

        row.map_or(
            Ok(None),
            |(
                route_json,
                title,
                launch_json,
                manifest_json,
                topology_json,
                screens_json,
                saved_at_ms,
            )| {
                Ok(Some(SavedNativeSession {
                    session_id,
                    route: serde_json::from_str(&route_json)?,
                    title,
                    launch: serde_json::from_str(&launch_json)?,
                    manifest: serde_json::from_str(&manifest_json)?,
                    topology: serde_json::from_str(&topology_json)?,
                    screens: serde_json::from_str(&screens_json)?,
                    saved_at_ms,
                }))
            },
        )
    }

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

    pub fn save_timestamp_ms() -> Result<i64, PersistenceError> {
        Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as i64)
    }
}
