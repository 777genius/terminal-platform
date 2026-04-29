use std::{
    fs,
    path::{Path, PathBuf},
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
    initialize_connection(&mut connection, config)?;
    Ok(connection)
}

pub fn initialize_connection(
    connection: &mut SqliteConnection,
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

    run_embedded_migrations(connection)?;
    ensure_db_identity(connection, config)?;

    Ok(())
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
            diesel::update(terminal_db_identity::table.filter(terminal_db_identity::id.eq(1)))
                .set((
                    terminal_db_identity::updated_at_ms.eq(now),
                    terminal_db_identity::sqlite_version.eq(sqlite_version(connection)?),
                ))
                .execute(connection)?;
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
