use diesel::sqlite::SqliteConnection;
use serde_json::Value;

use crate::v2::{TerminalPersistenceV2Config, TerminalPersistenceV2Error};

use super::pragmas::{
    sqlite_compile_options, sqlite_foreign_keys_enabled, sqlite_journal_mode,
    sqlite_synchronous_code, sqlite_version, sqlite_wal_autocheckpoint_pages,
};

pub(super) fn connection_diagnostics_json(
    connection: &mut SqliteConnection,
    config: &TerminalPersistenceV2Config,
) -> Result<Value, TerminalPersistenceV2Error> {
    Ok(serde_json::json!({
        "diagnostic_kind": "sqlite_startup",
        "sqlite_version": sqlite_version(connection)?,
        "journal_mode": sqlite_journal_mode(connection)?,
        "synchronous_code": sqlite_synchronous_code(connection)?,
        "configured_synchronous": config.durability_profile.sqlite_synchronous(),
        "foreign_keys": sqlite_foreign_keys_enabled(connection)?,
        "wal_autocheckpoint_pages": sqlite_wal_autocheckpoint_pages(connection)?,
        "configured_wal_autocheckpoint_pages": config.wal_autocheckpoint_pages,
        "configured_busy_timeout_ms": config.busy_timeout_ms,
        "compile_options": sqlite_compile_options(connection)?,
    }))
}
