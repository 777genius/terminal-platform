mod dedupe;
mod finalize;
mod input;
mod writes;

pub(super) use input::AppendStreamSegmentTransaction;

use super::super::super::*;
use dedupe::reuse_capture_receipt_if_possible;
use finalize::{final_event_seq, finalize_stream_segment};
use writes::{
    enqueue_projection_outbox, insert_buffer_mode_transition_rows, insert_primary_event_row,
    insert_receipt_if_needed, insert_stream_segment_row,
};

pub(super) fn append_stream_segment_transaction(
    connection: &mut SqliteConnection,
    tx: AppendStreamSegmentTransaction<'_>,
) -> Result<StreamSegmentReceipt, TerminalPersistenceV2Error> {
    if let Some(receipt) = reuse_capture_receipt_if_possible(connection, &tx)? {
        return Ok(receipt);
    }

    ensure_active_writer(connection, &tx.input.writer_generation, tx.now)?;
    let commit = allocate_commit(
        connection,
        &tx.input.session_id,
        "stream_segment",
        &tx.input.writer_generation,
        tx.occurred_at_ms,
        tx.now,
        None,
    )?;
    let cursor =
        load_stream_cursor(connection, &tx.input.session_id, &tx.input.pane_id, tx.stream_id)?;
    let event_seq_low = cursor.next_event_seq;
    let event_seq_high = cursor.next_event_seq;
    let byte_low = cursor.next_byte_seq;
    let byte_high = cursor.next_byte_seq + tx.payload_len;
    let segment_id = new_id();
    let event_id = new_id();
    let capture_semantics =
        tx.input.capture_semantics.clone().unwrap_or_else(|| "raw_vt_stream".to_string());
    validate_capture_semantics_domain(&capture_semantics)?;
    let event_type = tx.input.event_type.clone().unwrap_or_else(|| "terminal_output".to_string());
    let payload_json = tx.input.payload_json.as_ref().map(serde_json::to_string).transpose()?;
    let payload_schema_id =
        payload_json.as_ref().map(|_| payload_schema_id_for_journal_event(&event_type).to_string());
    let final_event_seq = final_event_seq(event_seq_high, tx.buffer_mode_transitions.len())?;

    insert_stream_segment_row(
        connection,
        &tx,
        &commit.id,
        &segment_id,
        event_seq_low,
        event_seq_high,
        byte_low,
        byte_high,
        &capture_semantics,
    )?;
    if tx.fail_after_segment_insert {
        return Err(TerminalPersistenceV2Error::InvalidData(
            "failpoint stream_segment_after_segment_insert".to_string(),
        ));
    }

    insert_primary_event_row(
        connection,
        &tx,
        &commit.id,
        &event_id,
        event_seq_low,
        byte_low,
        byte_high,
        event_type,
        payload_json,
        payload_schema_id,
        &capture_semantics,
    )?;
    insert_buffer_mode_transition_rows(
        connection,
        &tx,
        &commit.id,
        &segment_id,
        event_seq_low,
        byte_low,
        byte_high,
        &capture_semantics,
    )?;
    enqueue_projection_outbox(
        connection,
        &tx,
        &commit.id,
        event_seq_low,
        final_event_seq,
        byte_low,
        byte_high,
    )?;
    insert_receipt_if_needed(connection, &tx, &commit.id)?;
    finalize_stream_segment(
        connection,
        &cursor.id,
        &tx.input.pane_id,
        final_event_seq,
        byte_high,
        tx.now,
    )?;

    Ok(StreamSegmentReceipt {
        commit_id: commit.id,
        commit_seq: commit.commit_seq,
        segment_id,
        event_id,
        event_seq_low,
        event_seq_high,
        byte_low,
        byte_high,
        checksum: tx.payload_checksum.to_string(),
    })
}
