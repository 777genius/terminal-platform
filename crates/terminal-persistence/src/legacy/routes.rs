use rusqlite::{OptionalExtension, params};
use terminal_domain::SessionId;
use uuid::Uuid;

use super::{PersistenceError, SessionRouteRecord, SqliteSessionStore};

impl SqliteSessionStore {
    pub fn upsert_session_route(
        &self,
        record: &SessionRouteRecord,
    ) -> Result<(), PersistenceError> {
        let mut connection = self.open_connection()?;
        let transaction = connection.transaction()?;
        transaction.execute(
            "
            INSERT INTO session_routes (
                session_id,
                route_json,
                route_fingerprint
            ) VALUES (?1, ?2, ?3)
            ON CONFLICT(session_id) DO UPDATE SET
                route_json = excluded.route_json,
                route_fingerprint = excluded.route_fingerprint
            ",
            params![
                record.session_id.0.to_string(),
                serde_json::to_string(&record.route)?,
                record.route_fingerprint,
            ],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn load_session_route(
        &self,
        session_id: SessionId,
    ) -> Result<Option<SessionRouteRecord>, PersistenceError> {
        let connection = self.open_connection()?;
        let row = connection
            .query_row(
                "
                SELECT route_json, route_fingerprint
                FROM session_routes
                WHERE session_id = ?1
                ",
                params![session_id.0.to_string()],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;

        row.map_or(Ok(None), |(route_json, route_fingerprint)| {
            Ok(Some(SessionRouteRecord {
                session_id,
                route: serde_json::from_str(&route_json)?,
                route_fingerprint,
            }))
        })
    }

    pub fn load_session_route_by_fingerprint(
        &self,
        route_fingerprint: &str,
    ) -> Result<Option<SessionRouteRecord>, PersistenceError> {
        let connection = self.open_connection()?;
        let row = connection
            .query_row(
                "
                SELECT session_id, route_json
                FROM session_routes
                WHERE route_fingerprint = ?1
                ",
                params![route_fingerprint],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;

        row.map_or(Ok(None), |(session_id, route_json)| {
            Ok(Some(SessionRouteRecord {
                session_id: SessionId::from(Uuid::parse_str(&session_id).map_err(|error| {
                    PersistenceError::InvalidData(format!(
                        "invalid session route id `{session_id}` - {error}"
                    ))
                })?),
                route: serde_json::from_str(&route_json)?,
                route_fingerprint: route_fingerprint.to_string(),
            }))
        })
    }
}
