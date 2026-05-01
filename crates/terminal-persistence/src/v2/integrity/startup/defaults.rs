use super::super::super::*;
use super::payload_schemas::seed_payload_schemas;

pub(in crate::v2) fn verify_seeded_defaults(
    connection: &mut SqliteConnection,
) -> Result<(), TerminalPersistenceV2Error> {
    let identity = terminal_db_identity::table
        .select(DbIdentityProbeRow::as_select())
        .first::<DbIdentityProbeRow>(connection)
        .optional()?;
    if identity.is_none() {
        return Err(TerminalPersistenceV2Error::InvalidData(
            "terminal_db_identity was not initialized".to_string(),
        ));
    }

    let gate_count: i64 = terminal_feature_gates::table.count().get_result(connection)?;
    if gate_count == 0 {
        return Err(TerminalPersistenceV2Error::InvalidData(
            "terminal_feature_gates seed rows are missing".to_string(),
        ));
    }

    seed_payload_schemas(connection, current_time_ms())?;
    let schema_count: i64 = terminal_payload_schemas::table.count().get_result(connection)?;
    if schema_count == 0 {
        return Err(TerminalPersistenceV2Error::InvalidData(
            "terminal_payload_schemas seed rows are missing".to_string(),
        ));
    }

    Ok(())
}
