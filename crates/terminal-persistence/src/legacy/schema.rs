use std::{sync::Mutex, time::Duration};

use rusqlite::Connection;
use rusqlite_migration::{M, Migrations};

use super::{PersistenceError, SqliteSessionStore, retry::retry_persistence_operation};

fn migrations() -> Migrations<'static> {
    Migrations::new(vec![
        M::up(
            "
            CREATE TABLE IF NOT EXISTS native_saved_sessions (
                session_id TEXT PRIMARY KEY,
                route_json TEXT NOT NULL,
                title TEXT,
                launch_json TEXT,
                manifest_json TEXT NOT NULL DEFAULT '{\"format_version\":1,\"binary_version\":\"0.1.0-dev\",\"protocol_major\":0,\"protocol_minor\":1}',
                topology_json TEXT NOT NULL,
                screens_json TEXT NOT NULL,
                saved_at_ms INTEGER NOT NULL
            );
            ",
        ),
        // Keep migration cardinality stable for existing local stores that already advanced
        // to migration index 2 in earlier development builds.
        M::up("SELECT 1;"),
        M::up(
            "
            CREATE TABLE IF NOT EXISTS session_routes (
                session_id TEXT PRIMARY KEY,
                route_json TEXT NOT NULL,
                route_fingerprint TEXT NOT NULL UNIQUE
            );
            ",
        ),
    ])
}

impl SqliteSessionStore {
    pub(super) fn ensure_schema(&self) -> Result<(), PersistenceError> {
        let _guard = legacy_schema_lock().lock().map_err(|_| {
            PersistenceError::InvalidData("legacy schema lock poisoned".to_string())
        })?;
        let mut connection = Connection::open(&self.path)?;
        connection.busy_timeout(Duration::from_millis(5_000))?;
        migrations().to_latest(&mut connection)?;
        ensure_manifest_column(&connection)?;
        Ok(())
    }

    pub(super) fn open_connection(&self) -> Result<Connection, PersistenceError> {
        retry_persistence_operation(|| self.ensure_schema())?;
        let connection = Connection::open(&self.path)?;
        connection.busy_timeout(Duration::from_millis(5_000))?;
        Ok(connection)
    }
}

fn legacy_schema_lock() -> &'static Mutex<()> {
    static LOCK: Mutex<()> = Mutex::new(());
    &LOCK
}

fn ensure_manifest_column(connection: &Connection) -> Result<(), PersistenceError> {
    let mut statement = connection.prepare("PRAGMA table_info(native_saved_sessions)")?;
    let columns = statement.query_map([], |row| row.get::<_, String>(1))?;
    for column in columns {
        if column? == "manifest_json" {
            return Ok(());
        }
    }

    let alter = connection.execute(
        "
        ALTER TABLE native_saved_sessions
        ADD COLUMN manifest_json TEXT NOT NULL DEFAULT '{\"format_version\":1,\"binary_version\":\"0.1.0-dev\",\"protocol_major\":0,\"protocol_minor\":1}';
        ",
        [],
    );
    match alter {
        Ok(_) => Ok(()),
        Err(error) if duplicate_column_error(&error) => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn duplicate_column_error(error: &rusqlite::Error) -> bool {
    matches!(error, rusqlite::Error::SqliteFailure(_, Some(message)) if message.contains("duplicate column name"))
}
