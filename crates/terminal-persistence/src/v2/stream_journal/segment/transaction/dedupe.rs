use super::super::super::super::*;
use super::input::AppendStreamSegmentTransaction;

pub(super) fn reuse_capture_receipt_if_possible(
    connection: &mut SqliteConnection,
    tx: &AppendStreamSegmentTransaction<'_>,
) -> Result<Option<StreamSegmentReceipt>, TerminalPersistenceV2Error> {
    let (Some(source_kind), Some(source_event_id_hash)) =
        (tx.capture_source_kind.as_deref(), tx.source_event_id_hash.as_deref())
    else {
        return Ok(None);
    };
    let Some(receipt) =
        load_capture_receipt(connection, &tx.input.session_id, source_kind, source_event_id_hash)?
    else {
        return Ok(None);
    };

    if receipt.source_payload_hash != tx.payload_checksum {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "capture receipt payload hash mismatch for source_kind={source_kind}"
        )));
    }
    stream_segment_receipt_from_capture_receipt(connection, &receipt).map(Some)
}
