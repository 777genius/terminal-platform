use super::super::super::*;
use super::super::support::*;

#[test]
fn dedupes_retried_stream_segment_capture_by_source_event_id() {
    let store = test_store("stream-retry-dedupe");
    let (session_id, pane_id, writer) = session_and_pane(&store);
    let mut input = StreamSegmentInput::terminal_output(
        session_id.clone(),
        pane_id.clone(),
        writer.id,
        b"cargo test\r\n".to_vec(),
    );
    input.source_event_id_hash = Some(blake3_hash_text("runtime-output-seq:42"));

    let first = store.append_stream_segment(input.clone()).expect("first capture should persist");
    let retry = store.append_stream_segment(input).expect("retry should return existing receipt");
    let segments =
        store.list_stream_segments(&session_id, &pane_id, 1, 10).expect("segments should list");

    assert_eq!(retry, first);
    assert_eq!(segments.len(), 1);
    assert_eq!(segments[0].payload, b"cargo test\r\n");
}

#[test]
fn rejects_retry_with_same_source_event_id_and_different_payload() {
    let store = test_store("stream-retry-conflict");
    let (session_id, pane_id, writer) = session_and_pane(&store);
    let mut input = StreamSegmentInput::terminal_output(
        session_id.clone(),
        pane_id.clone(),
        writer.id.clone(),
        b"first\r\n".to_vec(),
    );
    input.source_event_id_hash = Some(blake3_hash_text("runtime-output-seq:43"));
    store.append_stream_segment(input.clone()).expect("first capture should persist");

    input.writer_generation = writer.id;
    input.payload = b"changed\r\n".to_vec();
    let error = store.append_stream_segment(input).expect_err("conflicting retry should fail");
    let segments =
        store.list_stream_segments(&session_id, &pane_id, 1, 10).expect("segments should list");

    assert!(
        matches!(error, TerminalPersistenceV2Error::InvalidData(message) if message.contains("payload hash mismatch"))
    );
    assert_eq!(segments.len(), 1);
    assert_eq!(segments[0].payload, b"first\r\n");
}

#[test]
fn stream_segment_failpoint_rolls_back_partial_writer_transaction() {
    let mut config = TerminalPersistenceV2Config::test();
    config.failpoints.stream_segment_after_segment_insert = true;
    let path = std::env::temp_dir()
        .join(format!("terminal-persistence-v2-stream-failpoint-{}.sqlite3", Uuid::new_v4()));
    let store = TerminalPersistenceV2::open_with_config(path, config)
        .expect("store should open with failpoint config");
    let (session_id, pane_id, writer) = session_and_pane(&store);

    let error = store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id.clone(),
            writer.id,
            b"partial write should roll back\r\n".to_vec(),
        ))
        .expect_err("failpoint should abort stream segment append");

    assert!(
        matches!(error, TerminalPersistenceV2Error::InvalidData(message) if message.contains("stream_segment_after_segment_insert"))
    );
    let mut connection = store.connection().expect("connection should open");
    let segment_count = terminal_stream_segments::table
        .filter(terminal_stream_segments::session_id.eq(&session_id))
        .count()
        .get_result::<i64>(&mut connection)
        .expect("segment count should load");
    let event_count = terminal_journal_events::table
        .filter(terminal_journal_events::session_id.eq(&session_id))
        .count()
        .get_result::<i64>(&mut connection)
        .expect("event count should load");
    let outbox_count = terminal_outbox_messages::table
        .count()
        .get_result::<i64>(&mut connection)
        .expect("outbox count should load");
    let cursor = load_stream_cursor(&mut connection, &session_id, &pane_id, DEFAULT_STREAM_ID)
        .expect("stream cursor should load");
    let pane_last_event_seq = terminal_panes::table
        .filter(terminal_panes::id.eq(&pane_id))
        .select(terminal_panes::last_event_seq)
        .first::<i64>(&mut connection)
        .expect("pane cursor should load");

    assert_eq!(segment_count, 0);
    assert_eq!(event_count, 0);
    assert_eq!(outbox_count, 0);
    assert_eq!(cursor.next_event_seq, 1);
    assert_eq!(cursor.next_byte_seq, 0);
    assert_eq!(pane_last_event_seq, 0);
}

#[test]
fn stream_segment_storage_full_failpoint_records_pressure_without_history_mutation() {
    let mut config = TerminalPersistenceV2Config::test();
    config.failpoints.stream_segment_before_transaction_storage_full = true;
    let path = std::env::temp_dir()
        .join(format!("terminal-persistence-v2-stream-storage-full-{}.sqlite3", Uuid::new_v4()));
    let store = TerminalPersistenceV2::open_with_config(path, config)
        .expect("store should open with failpoint config");
    let (session_id, pane_id, writer) = session_and_pane(&store);

    let error = store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id,
            writer.id,
            b"storage full should fail closed\r\n".to_vec(),
        ))
        .expect_err("storage-full failpoint should abort stream segment append");
    let mut connection = store.connection().expect("connection should open");
    let segment_count = terminal_stream_segments::table
        .filter(terminal_stream_segments::session_id.eq(&session_id))
        .count()
        .get_result::<i64>(&mut connection)
        .expect("segment count should load");
    let outbox_count = terminal_outbox_messages::table
        .count()
        .get_result::<i64>(&mut connection)
        .expect("outbox count should load");
    let (state, action_taken, reason, metadata_json) = terminal_storage_pressure_events::table
        .order(terminal_storage_pressure_events::created_at_ms.desc())
        .select((
            terminal_storage_pressure_events::state,
            terminal_storage_pressure_events::action_taken,
            terminal_storage_pressure_events::reason,
            terminal_storage_pressure_events::metadata_json,
        ))
        .first::<(String, String, Option<String>, Option<String>)>(&mut connection)
        .expect("storage pressure event should persist");
    let metadata: Value = serde_json::from_str(
        metadata_json.as_deref().expect("storage pressure metadata should exist"),
    )
    .expect("storage pressure metadata should be json");

    assert!(
        matches!(error, TerminalPersistenceV2Error::InvalidData(message) if message.contains("stream_segment_before_transaction_storage_full"))
    );
    assert_eq!(segment_count, 0);
    assert_eq!(outbox_count, 0);
    assert_eq!(state, "full");
    assert_eq!(action_taken, "fail_closed");
    assert_eq!(reason.as_deref(), Some("synthetic_sqlite_full"));
    assert_eq!(metadata["operation"], "append_stream_segment");
    assert_eq!(metadata["no_silent_delete"], true);
    assert_eq!(metadata["canonical_history_preserved"], true);
}
