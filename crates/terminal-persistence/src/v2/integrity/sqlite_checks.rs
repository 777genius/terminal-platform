use super::super::*;

pub(in crate::v2) fn run_quick_check(
    connection: &mut SqliteConnection,
) -> Result<Vec<String>, TerminalPersistenceV2Error> {
    diesel::sql_query("PRAGMA quick_check")
        .load::<QuickCheckRow>(connection)
        .map(|rows| rows.into_iter().map(|row| row.quick_check).collect())
        .map_err(Into::into)
}

pub(in crate::v2) fn run_passive_wal_checkpoint(
    connection: &mut SqliteConnection,
) -> Result<Value, TerminalPersistenceV2Error> {
    let rows =
        diesel::sql_query("PRAGMA wal_checkpoint(PASSIVE)").load::<WalCheckpointRow>(connection)?;
    let Some(row) = rows.into_iter().next() else {
        return Err(TerminalPersistenceV2Error::InvalidData(
            "wal_checkpoint returned no rows".to_string(),
        ));
    };

    Ok(serde_json::json!({
        "mode": "PASSIVE",
        "busy": row.busy,
        "log_frames": row.log,
        "checkpointed_frames": row.checkpointed,
    }))
}

pub(in crate::v2) fn run_foreign_key_check(
    connection: &mut SqliteConnection,
) -> Result<Vec<ForeignKeyCheckRow>, TerminalPersistenceV2Error> {
    diesel::sql_query(
        "SELECT \"table\" AS table_name, rowid, parent, fkid FROM pragma_foreign_key_check",
    )
    .load::<ForeignKeyCheckRow>(connection)
    .map_err(Into::into)
}
