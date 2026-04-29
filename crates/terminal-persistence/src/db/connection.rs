use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

use diesel::{
    Connection, RunQueryDsl, SelectableHelper, connection::SimpleConnection, dsl::insert_into,
    prelude::*, sqlite::SqliteConnection,
};

use crate::{
    db::{migrations::run_embedded_migrations, schema::terminal_db_identity},
    v2::{
        TERMINAL_PERSISTENCE_APP_ID, TerminalDbIdentityRow, TerminalPersistenceV2Config,
        TerminalPersistenceV2Error,
    },
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

    run_embedded_migrations(connection)?;
    ensure_db_identity(connection, config)?;
    mark_process_initialized(init_key)?;

    Ok(())
}

fn connection_init_lock() -> &'static Mutex<()> {
    static LOCK: Mutex<()> = Mutex::new(());
    &LOCK
}

fn initialized_databases() -> &'static Mutex<HashSet<PathBuf>> {
    static INITIALIZED_DATABASES: OnceLock<Mutex<HashSet<PathBuf>>> = OnceLock::new();
    INITIALIZED_DATABASES.get_or_init(|| Mutex::new(HashSet::new()))
}

fn is_process_initialized(path: &Path) -> bool {
    initialized_databases().lock().map(|initialized| initialized.contains(path)).unwrap_or(false)
}

fn mark_process_initialized(path: PathBuf) -> Result<(), TerminalPersistenceV2Error> {
    initialized_databases()
        .lock()
        .map_err(|_| {
            TerminalPersistenceV2Error::InvalidData(
                "terminal persistence init registry poisoned".to_string(),
            )
        })?
        .insert(path);
    Ok(())
}

fn database_init_key(path: &Path) -> Result<PathBuf, TerminalPersistenceV2Error> {
    path.canonicalize()
        .or_else(|_| Ok::<PathBuf, std::io::Error>(path.to_path_buf()))
        .map_err(Into::into)
}

fn ensure_db_identity(
    connection: &mut SqliteConnection,
    config: &TerminalPersistenceV2Config,
) -> Result<(), TerminalPersistenceV2Error> {
    connection.batch_execute(&format!("PRAGMA application_id = {TERMINAL_PERSISTENCE_APP_ID};"))?;
    let now = config.clock.now_ms();
    let existing = terminal_db_identity::table
        .filter(terminal_db_identity::id.eq(1))
        .select(TerminalDbIdentityRow::as_select())
        .first::<TerminalDbIdentityRow>(connection)
        .optional()?;

    match existing {
        Some(identity)
            if identity.product == "terminal-platform"
                && identity.schema_family == "terminal_persistence_v2" =>
        {
            // Opening a hot terminal-history database must stay read-only after
            // identity is established. Raw output capture opens short-lived
            // Diesel connections often, and an identity heartbeat would contend
            // with real history writes under SQLite's single-writer lock.
        }
        Some(identity) => {
            return Err(TerminalPersistenceV2Error::IdentityMismatch {
                product: identity.product,
                schema_family: identity.schema_family,
            });
        }
        None => {
            let row = TerminalDbIdentityRow {
                id: 1,
                product: "terminal-platform".to_string(),
                schema_family: "terminal_persistence_v2".to_string(),
                created_at_ms: now,
                updated_at_ms: now,
                app_version: Some(env!("CARGO_PKG_VERSION").to_string()),
                diesel_version: Some("2.3.8".to_string()),
                sqlite_version: Some(sqlite_version(connection)?),
                notes: None,
            };
            insert_into(terminal_db_identity::table).values(&row).execute(connection)?;
        }
    }

    Ok(())
}

pub fn sqlite_version(
    connection: &mut SqliteConnection,
) -> Result<String, TerminalPersistenceV2Error> {
    diesel::select(diesel::dsl::sql::<diesel::sql_types::Text>("sqlite_version()"))
        .get_result(connection)
        .map_err(Into::into)
}

pub fn sqlite_application_id(
    connection: &mut SqliteConnection,
) -> Result<i32, TerminalPersistenceV2Error> {
    diesel::sql_query("PRAGMA application_id")
        .load::<ApplicationIdPragmaRow>(connection)?
        .into_iter()
        .next()
        .map(|row| row.application_id)
        .ok_or_else(|| {
            TerminalPersistenceV2Error::InvalidData("missing application_id".to_string())
        })
}

fn path_to_database_url(path: &Path) -> Result<String, TerminalPersistenceV2Error> {
    path.canonicalize()
        .or_else(|_| Ok::<PathBuf, std::io::Error>(path.to_path_buf()))?
        .to_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            TerminalPersistenceV2Error::InvalidData("database path is not UTF-8".to_string())
        })
}

#[derive(Debug, QueryableByName)]
struct ApplicationIdPragmaRow {
    #[diesel(sql_type = diesel::sql_types::Integer)]
    application_id: i32,
}
