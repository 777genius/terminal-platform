use super::super::super::*;
use super::super::support::*;

#[test]
fn creates_session_pane_and_reopens_with_history_cursor() {
    let store = test_store("session-pane");
    let path = store.path().to_path_buf();
    let session_id = store.create_session(SessionInput::new(route())).expect("session should save");
    let pane_id =
        store.create_pane(PaneInput::new(session_id.clone(), 30, 120)).expect("pane should save");

    let reopened =
        TerminalPersistenceV2::open_with_config(path, TerminalPersistenceV2Config::test())
            .expect("store should reopen");
    let mut connection = reopened.connection().expect("connection should open");
    let cursor = load_stream_cursor(&mut connection, &session_id, &pane_id, DEFAULT_STREAM_ID)
        .expect("stream cursor should exist");

    assert_eq!(cursor.next_event_seq, 1);
    assert_eq!(cursor.next_byte_seq, 0);
}

#[test]
fn enforces_single_active_writer_generation() {
    let store = test_store("writer-generation");

    let first =
        store.acquire_writer_generation("process-a", 60_000).expect("first writer should acquire");
    let second = store.acquire_writer_generation("process-b", 60_000);

    assert!(matches!(second, Err(TerminalPersistenceV2Error::WriterAlreadyActive)));
    store.release_writer_generation(&first.id).expect("writer should release");
    store
        .acquire_writer_generation("process-b", 60_000)
        .expect("new writer should acquire after release");
}

#[test]
fn writer_generation_records_clock_anchors() {
    let store = test_store("writer-clock-anchors");
    let writer =
        store.acquire_writer_generation("process-a", 60_000).expect("writer should acquire");

    store.heartbeat_writer_generation(&writer.id, 60_000).expect("writer heartbeat should persist");
    store.release_writer_generation(&writer.id).expect("writer should release");

    let mut connection = store.connection().expect("connection should open");
    let anchors = terminal_clock_anchors::table
        .filter(terminal_clock_anchors::writer_generation.eq(&writer.id))
        .order(terminal_clock_anchors::created_at_ms.asc())
        .select((
            terminal_clock_anchors::source,
            terminal_clock_anchors::wall_time_ms,
            terminal_clock_anchors::monotonic_ms,
        ))
        .load::<(String, i64, i64)>(&mut connection)
        .expect("clock anchors should load");
    let sources = anchors.iter().map(|(source, _, _)| source.as_str()).collect::<Vec<_>>();

    assert_eq!(sources, vec!["writer_acquire", "writer_heartbeat", "writer_release"]);
    assert!(anchors.iter().all(|(_, wall_time_ms, _)| *wall_time_ms > 0));
    assert!(anchors.iter().all(|(_, _, monotonic_ms)| *monotonic_ms >= 0));
}
