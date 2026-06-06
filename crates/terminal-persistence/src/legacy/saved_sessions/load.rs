use rusqlite::{OptionalExtension, params};

use super::super::{PersistenceError, SavedNativeSession, SqliteSessionStore};

type SavedNativeSessionPayloadRow = (String, Option<String>, String, String, String, String, i64);

impl SqliteSessionStore {
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
                decode_saved_native_session_payload_row,
            )
            .optional()?;

        row.map(|row| decode_saved_native_session(session_id, row)).transpose()
    }
}

fn decode_saved_native_session_payload_row(
    row: &rusqlite::Row<'_>,
) -> Result<SavedNativeSessionPayloadRow, rusqlite::Error> {
    Ok((
        row.get::<_, String>(0)?,
        row.get::<_, Option<String>>(1)?,
        row.get::<_, String>(2)?,
        row.get::<_, String>(3)?,
        row.get::<_, String>(4)?,
        row.get::<_, String>(5)?,
        row.get::<_, i64>(6)?,
    ))
}

fn decode_saved_native_session(
    session_id: terminal_domain::SessionId,
    (
        route_json,
        title,
        launch_json,
        manifest_json,
        topology_json,
        screens_json,
        saved_at_ms,
    ): SavedNativeSessionPayloadRow,
) -> Result<SavedNativeSession, PersistenceError> {
    Ok(SavedNativeSession {
        session_id,
        route: serde_json::from_str(&route_json)?,
        title,
        launch: serde_json::from_str(&launch_json)?,
        manifest: serde_json::from_str(&manifest_json)?,
        topology: serde_json::from_str(&topology_json)?,
        screens: serde_json::from_str(&screens_json)?,
        saved_at_ms,
    })
}
