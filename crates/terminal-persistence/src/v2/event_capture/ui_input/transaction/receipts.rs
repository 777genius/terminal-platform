use super::super::super::super::*;
use super::context::UiInputTransaction;

pub(super) fn reuse_ui_input_receipt_if_possible(
    connection: &mut SqliteConnection,
    tx: &UiInputTransaction<'_>,
) -> Result<bool, TerminalPersistenceV2Error> {
    let (Some(source_kind), Some(source_event_id_hash)) =
        (tx.capture_source_kind.as_deref(), tx.source_event_id_hash.as_deref())
    else {
        return Ok(false);
    };
    let Some(receipt) =
        load_capture_receipt(connection, &tx.input.session_id, source_kind, source_event_id_hash)?
    else {
        return Ok(false);
    };
    if receipt.source_payload_hash != tx.payload_hash {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "ui input receipt payload hash mismatch for source_kind={source_kind}"
        )));
    }
    Ok(true)
}

pub(super) fn insert_ui_input_capture_receipt(
    connection: &mut SqliteConnection,
    tx: &UiInputTransaction<'_>,
    commit_id: &str,
) -> Result<(), TerminalPersistenceV2Error> {
    if let (Some(source_kind), Some(source_event_id_hash)) =
        (tx.capture_source_kind.as_deref(), tx.source_event_id_hash.as_deref())
    {
        let receipt = NewCaptureReceiptRow {
            id: new_id(),
            session_id: tx.input.session_id.clone(),
            commit_id: Some(commit_id.to_string()),
            source_kind: source_kind.to_string(),
            source_event_id_hash: source_event_id_hash.to_string(),
            source_payload_hash: tx.payload_hash.clone(),
            received_at_ms: tx.now,
            created_at_ms: tx.now,
            metadata_json: None,
        };
        insert_into(terminal_capture_receipts::table).values(&receipt).execute(connection)?;
    }
    Ok(())
}
