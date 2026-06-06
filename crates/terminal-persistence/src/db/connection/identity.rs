use diesel::{
    OptionalExtension, RunQueryDsl, SelectableHelper, connection::SimpleConnection,
    dsl::insert_into, prelude::*, sqlite::SqliteConnection,
};

use crate::{
    db::schema::terminal_db_identity,
    v2::{
        TERMINAL_PERSISTENCE_APP_ID, TerminalDbIdentityRow, TerminalPersistenceV2Config,
        TerminalPersistenceV2Error,
    },
};

use super::{diagnostics::connection_diagnostics_json, pragmas::sqlite_version};

pub(super) fn ensure_db_identity(
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
            // Avoid heartbeat writes on hot opens. A legacy identity without
            // diagnostics is upgraded once so support bundles can prove the
            // actual SQLite/WAL startup profile later.
            if identity.notes.is_none() {
                let notes = connection_diagnostics_json(connection, config)?;
                diesel::update(terminal_db_identity::table.filter(terminal_db_identity::id.eq(1)))
                    .set((
                        terminal_db_identity::updated_at_ms.eq(now),
                        terminal_db_identity::sqlite_version.eq(Some(sqlite_version(connection)?)),
                        terminal_db_identity::notes.eq(Some(serde_json::to_string(&notes)?)),
                    ))
                    .execute(connection)?;
            }
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
                notes: Some(serde_json::to_string(&connection_diagnostics_json(
                    connection, config,
                )?)?),
            };
            insert_into(terminal_db_identity::table).values(&row).execute(connection)?;
        }
    }

    Ok(())
}
