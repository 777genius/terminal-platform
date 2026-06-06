use super::super::super::super::*;
use super::super::{
    rows::{
        PrimaryEventRowInput, StreamSegmentRowInput, new_primary_journal_event_row,
        new_stream_segment_row,
    },
    side_effects::{
        CaptureReceiptInput, ProjectionOutboxInput, enqueue_pane_history_projection,
        insert_capture_receipt,
    },
    transitions::{BufferModeTransitionInput, insert_buffer_mode_transition_events},
};
use super::input::AppendStreamSegmentTransaction;

pub(super) fn insert_stream_segment_row(
    connection: &mut SqliteConnection,
    tx: &AppendStreamSegmentTransaction<'_>,
    commit_id: &str,
    segment_id: &str,
    event_seq_low: i64,
    event_seq_high: i64,
    byte_low: i64,
    byte_high: i64,
    capture_semantics: &str,
) -> Result<(), TerminalPersistenceV2Error> {
    let segment = new_stream_segment_row(StreamSegmentRowInput {
        segment_id,
        session_id: &tx.input.session_id,
        pane_id: &tx.input.pane_id,
        commit_id,
        stream_id: tx.stream_id,
        event_seq_low,
        event_seq_high,
        byte_low,
        byte_high,
        payload: &tx.input.payload,
        payload_len: tx.payload_len,
        payload_checksum: tx.payload_checksum,
        capture_semantics,
        writer_generation: &tx.input.writer_generation,
        metadata_json: tx.metadata_json.clone(),
        now: tx.now,
    });
    insert_into(terminal_stream_segments::table).values(&segment).execute(connection)?;
    Ok(())
}

pub(super) fn insert_primary_event_row(
    connection: &mut SqliteConnection,
    tx: &AppendStreamSegmentTransaction<'_>,
    commit_id: &str,
    event_id: &str,
    event_seq: i64,
    byte_low: i64,
    byte_high: i64,
    event_type: String,
    payload_json: Option<String>,
    payload_schema_id: Option<String>,
    capture_semantics: &str,
) -> Result<(), TerminalPersistenceV2Error> {
    let event = new_primary_journal_event_row(PrimaryEventRowInput {
        event_id,
        session_id: &tx.input.session_id,
        pane_id: &tx.input.pane_id,
        commit_id,
        stream_id: tx.stream_id,
        event_seq,
        event_type,
        byte_low,
        byte_high,
        payload_json,
        payload_schema_id,
        source_event_id_hash: tx.source_event_id_hash.clone(),
        occurred_at_ms: tx.occurred_at_ms,
        now: tx.now,
        capture_semantics,
        trust_level: tx.input.trust_level.clone().unwrap_or_else(|| "captured".to_string()),
        metadata_json: tx.metadata_json.clone(),
    });
    insert_into(terminal_journal_events::table).values(&event).execute(connection)?;
    Ok(())
}

pub(super) fn insert_buffer_mode_transition_rows(
    connection: &mut SqliteConnection,
    tx: &AppendStreamSegmentTransaction<'_>,
    commit_id: &str,
    segment_id: &str,
    event_seq_low: i64,
    byte_low: i64,
    byte_high: i64,
    capture_semantics: &str,
) -> Result<(), TerminalPersistenceV2Error> {
    insert_buffer_mode_transition_events(
        connection,
        BufferModeTransitionInput {
            transitions: tx.buffer_mode_transitions,
            session_id: &tx.input.session_id,
            pane_id: &tx.input.pane_id,
            commit_id,
            stream_id: tx.stream_id,
            segment_id,
            event_seq_low,
            byte_low,
            byte_high,
            occurred_at_ms: tx.occurred_at_ms,
            now: tx.now,
            capture_semantics,
        },
    )
}

pub(super) fn enqueue_projection_outbox(
    connection: &mut SqliteConnection,
    tx: &AppendStreamSegmentTransaction<'_>,
    commit_id: &str,
    event_seq_low: i64,
    event_seq_high: i64,
    byte_low: i64,
    byte_high: i64,
) -> Result<(), TerminalPersistenceV2Error> {
    enqueue_pane_history_projection(
        connection,
        ProjectionOutboxInput {
            session_id: &tx.input.session_id,
            pane_id: &tx.input.pane_id,
            stream_id: tx.stream_id,
            commit_id,
            event_seq_low,
            event_seq_high,
            byte_low,
            byte_high,
            now: tx.now,
        },
    )
}

pub(super) fn insert_receipt_if_needed(
    connection: &mut SqliteConnection,
    tx: &AppendStreamSegmentTransaction<'_>,
    commit_id: &str,
) -> Result<(), TerminalPersistenceV2Error> {
    if let (Some(source_kind), Some(source_event_id_hash)) =
        (tx.capture_source_kind.as_deref(), tx.source_event_id_hash.as_deref())
    {
        insert_capture_receipt(
            connection,
            CaptureReceiptInput {
                session_id: &tx.input.session_id,
                commit_id,
                source_kind,
                source_event_id_hash,
                source_payload_hash: tx.payload_checksum,
                received_at_ms: tx.occurred_at_ms,
                created_at_ms: tx.now,
                metadata_json: tx.metadata_json.clone(),
            },
        )?;
    }
    Ok(())
}
