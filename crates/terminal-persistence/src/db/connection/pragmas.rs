use diesel::{RunQueryDsl, prelude::*, sqlite::SqliteConnection};

use crate::v2::TerminalPersistenceV2Error;

pub(super) fn sqlite_version(
    connection: &mut SqliteConnection,
) -> Result<String, TerminalPersistenceV2Error> {
    diesel::select(diesel::dsl::sql::<diesel::sql_types::Text>("sqlite_version()"))
        .get_result(connection)
        .map_err(Into::into)
}

pub(super) fn sqlite_application_id(
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

pub(super) fn sqlite_journal_mode(
    connection: &mut SqliteConnection,
) -> Result<String, TerminalPersistenceV2Error> {
    diesel::sql_query("PRAGMA journal_mode")
        .load::<JournalModePragmaRow>(connection)?
        .into_iter()
        .next()
        .map(|row| row.journal_mode)
        .ok_or_else(|| TerminalPersistenceV2Error::InvalidData("missing journal_mode".to_string()))
}

pub(super) fn sqlite_synchronous_code(
    connection: &mut SqliteConnection,
) -> Result<i32, TerminalPersistenceV2Error> {
    diesel::sql_query("PRAGMA synchronous")
        .load::<SynchronousPragmaRow>(connection)?
        .into_iter()
        .next()
        .map(|row| row.synchronous)
        .ok_or_else(|| TerminalPersistenceV2Error::InvalidData("missing synchronous".to_string()))
}

pub(super) fn sqlite_foreign_keys_enabled(
    connection: &mut SqliteConnection,
) -> Result<bool, TerminalPersistenceV2Error> {
    diesel::sql_query("PRAGMA foreign_keys")
        .load::<ForeignKeysPragmaRow>(connection)?
        .into_iter()
        .next()
        .map(|row| row.foreign_keys != 0)
        .ok_or_else(|| TerminalPersistenceV2Error::InvalidData("missing foreign_keys".to_string()))
}

pub(super) fn sqlite_wal_autocheckpoint_pages(
    connection: &mut SqliteConnection,
) -> Result<i32, TerminalPersistenceV2Error> {
    diesel::sql_query("PRAGMA wal_autocheckpoint")
        .load::<WalAutocheckpointPragmaRow>(connection)?
        .into_iter()
        .next()
        .map(|row| row.wal_autocheckpoint)
        .ok_or_else(|| {
            TerminalPersistenceV2Error::InvalidData("missing wal_autocheckpoint".to_string())
        })
}

pub(super) fn sqlite_compile_options(
    connection: &mut SqliteConnection,
) -> Result<Vec<String>, TerminalPersistenceV2Error> {
    Ok(diesel::sql_query("PRAGMA compile_options")
        .load::<CompileOptionPragmaRow>(connection)?
        .into_iter()
        .map(|row| row.compile_options)
        .collect())
}

#[derive(Debug, QueryableByName)]
struct ApplicationIdPragmaRow {
    #[diesel(sql_type = diesel::sql_types::Integer)]
    application_id: i32,
}

#[derive(Debug, QueryableByName)]
struct JournalModePragmaRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    journal_mode: String,
}

#[derive(Debug, QueryableByName)]
struct SynchronousPragmaRow {
    #[diesel(sql_type = diesel::sql_types::Integer)]
    synchronous: i32,
}

#[derive(Debug, QueryableByName)]
struct ForeignKeysPragmaRow {
    #[diesel(sql_type = diesel::sql_types::Integer)]
    foreign_keys: i32,
}

#[derive(Debug, QueryableByName)]
struct WalAutocheckpointPragmaRow {
    #[diesel(sql_type = diesel::sql_types::Integer)]
    wal_autocheckpoint: i32,
}

#[derive(Debug, QueryableByName)]
struct CompileOptionPragmaRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    compile_options: String,
}
