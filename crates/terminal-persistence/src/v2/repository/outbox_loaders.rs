use super::super::*;

pub(in crate::v2) fn touch_delivery_client(
    connection: &mut SqliteConnection,
    client_id: &str,
    now: i64,
) -> Result<(), TerminalPersistenceV2Error> {
    let updated =
        diesel::update(terminal_clients::table.filter(terminal_clients::id.eq(client_id)))
            .set(terminal_clients::last_seen_at_ms.eq(now))
            .execute(connection)?;
    if updated == 0 {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "delivery client not found: {client_id}"
        )));
    }
    Ok(())
}

pub(in crate::v2) fn load_delivery_offset(
    connection: &mut SqliteConnection,
    client_id: &str,
    session_id: &str,
    pane_id: &str,
    stream_id: &str,
) -> Result<Option<DeliveryOffsetRow>, TerminalPersistenceV2Error> {
    terminal_delivery_offsets::table
        .filter(terminal_delivery_offsets::client_id.eq(client_id))
        .filter(terminal_delivery_offsets::session_id.eq(session_id))
        .filter(terminal_delivery_offsets::pane_id.eq(Some(pane_id.to_string())))
        .filter(terminal_delivery_offsets::stream_id.eq(stream_id))
        .select(DeliveryOffsetRow::as_select())
        .first::<DeliveryOffsetRow>(connection)
        .optional()
        .map_err(Into::into)
}

pub(in crate::v2) fn load_outbox_message(
    connection: &mut SqliteConnection,
    id: &str,
) -> Result<OutboxMessageRow, TerminalPersistenceV2Error> {
    terminal_outbox_messages::table
        .filter(terminal_outbox_messages::id.eq(id))
        .select(OutboxMessageRow::as_select())
        .first::<OutboxMessageRow>(connection)
        .map_err(Into::into)
}

pub(in crate::v2) fn load_outbox_message_by_dedupe(
    connection: &mut SqliteConnection,
    dedupe_key: &str,
) -> Result<Option<OutboxMessageRow>, TerminalPersistenceV2Error> {
    terminal_outbox_messages::table
        .filter(terminal_outbox_messages::dedupe_key.eq(Some(dedupe_key.to_string())))
        .select(OutboxMessageRow::as_select())
        .first::<OutboxMessageRow>(connection)
        .optional()
        .map_err(Into::into)
}
