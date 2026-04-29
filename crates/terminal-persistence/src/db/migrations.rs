use diesel::sqlite::SqliteConnection;
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};

use crate::v2::TerminalPersistenceV2Error;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

pub fn run_embedded_migrations(
    connection: &mut SqliteConnection,
) -> Result<(), TerminalPersistenceV2Error> {
    connection
        .run_pending_migrations(MIGRATIONS)
        .map(|_| ())
        .map_err(|error| TerminalPersistenceV2Error::Migration(error.to_string()))
}
