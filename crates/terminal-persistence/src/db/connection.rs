mod diagnostics;
mod identity;
mod init_registry;
mod path;
mod pragmas;

use std::{fs, path::Path};

use diesel::{Connection, connection::SimpleConnection, sqlite::SqliteConnection};

use crate::v2::{
    TERMINAL_PERSISTENCE_APP_ID, TerminalPersistenceV2Config, TerminalPersistenceV2Error,
};

use self::{
    identity::ensure_db_identity,
    init_registry::{connection_init_lock, is_process_initialized, mark_process_initialized},
    path::{database_init_key, path_to_database_url},
    pragmas::sqlite_application_id,
};

pub fn establish_initialized_connection(
    path: &Path,
    config: &TerminalPersistenceV2Config,
) -> Result<SqliteConnection, TerminalPersistenceV2Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let database_url = path_to_database_url(path)?;
    let mut connection = SqliteConnection::establish(&database_url)?;
    initialize_connection(&mut connection, path, config)?;
    Ok(connection)
}

pub fn initialize_connection(
    connection: &mut SqliteConnection,
    path: &Path,
    config: &TerminalPersistenceV2Config,
) -> Result<(), TerminalPersistenceV2Error> {
    connection.batch_execute(&format!("PRAGMA busy_timeout = {};", config.busy_timeout_ms))?;
    connection.batch_execute("PRAGMA foreign_keys = ON;")?;
    let app_id = sqlite_application_id(connection)?;
    if app_id != 0 && app_id != TERMINAL_PERSISTENCE_APP_ID {
        return Err(TerminalPersistenceV2Error::WrongDatabase { application_id: app_id });
    }

    connection.batch_execute(&format!(
        "
        PRAGMA journal_mode = WAL;
        PRAGMA wal_autocheckpoint = {};
        PRAGMA temp_store = MEMORY;
        ",
        config.wal_autocheckpoint_pages
    ))?;
    connection.batch_execute(&format!(
        "PRAGMA synchronous = {};",
        config.durability_profile.sqlite_synchronous()
    ))?;

    let init_key = database_init_key(path)?;
    if is_process_initialized(&init_key) {
        return Ok(());
    }

    let _guard = connection_init_lock().lock().map_err(|_| {
        TerminalPersistenceV2Error::InvalidData(
            "terminal persistence init lock poisoned".to_string(),
        )
    })?;
    if is_process_initialized(&init_key) {
        return Ok(());
    }

    crate::db::migrations::run_embedded_migrations(connection)?;
    ensure_db_identity(connection, config)?;
    mark_process_initialized(init_key)?;

    Ok(())
}
