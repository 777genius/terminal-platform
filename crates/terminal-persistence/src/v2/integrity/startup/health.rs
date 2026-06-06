use super::super::super::*;

pub(in crate::v2) fn ensure_no_open_critical_health_records(
    connection: &mut SqliteConnection,
    session_id: Option<&str>,
    operation_kind: &str,
) -> Result<(), TerminalPersistenceV2Error> {
    let mut query = terminal_data_health_records::table
        .filter(terminal_data_health_records::severity.eq("critical"))
        .filter(terminal_data_health_records::action_state.ne("resolved"))
        .filter(terminal_data_health_records::action_state.ne("ignored"))
        .into_boxed();
    if let Some(session_id) = session_id {
        query = query.filter(
            terminal_data_health_records::session_id
                .is_null()
                .or(terminal_data_health_records::session_id.eq(Some(session_id.to_string()))),
        );
    }

    let record = query
        .select(DataHealthRecordRow::as_select())
        .first::<DataHealthRecordRow>(connection)
        .optional()?;
    if let Some(record) = record {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "{operation_kind} is blocked by open critical data health record {}",
            record.id
        )));
    }

    Ok(())
}
