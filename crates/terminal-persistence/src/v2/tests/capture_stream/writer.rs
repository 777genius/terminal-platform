use super::super::super::*;
use super::super::support::*;
use terminal_projection::ScreenSnapshot;

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

#[test]
fn screen_snapshot_event_uses_pane_event_high_water_not_projection_sequence() {
    let store = test_store("screen-projection-sequence-overflow");
    let session_id = SessionId::new();
    let pane_id = PaneId::new();
    let session_ref = session_id.0.to_string();
    let pane_ref = pane_id.0.to_string();

    store
        .upsert_runtime_session(SessionInput {
            id: Some(session_ref.clone()),
            route: route(),
            title: Some("zellij".to_string()),
            launch: None,
            source: Some("test".to_string()),
            durability_profile: None,
            retention_policy_id: None,
            private_mode: false,
            metadata: None,
        })
        .expect("session should upsert");
    store
        .upsert_runtime_pane(PaneInput {
            id: Some(pane_ref.clone()),
            session_id: session_ref.clone(),
            tab_id: None,
            stream_id: None,
            title: Some("zellij".to_string()),
            rows: 24,
            cols: 80,
            metadata: None,
        })
        .expect("pane should upsert");
    let writer =
        store.acquire_writer_generation("test-process", 60_000).expect("writer should acquire");
    let segment = store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_ref.clone(),
            pane_ref.clone(),
            writer.id.clone(),
            b"rendered history\n".to_vec(),
        ))
        .expect("segment should persist");
    store.release_writer_generation(&writer.id).expect("writer should release");

    let snapshot_id = store
        .record_screen_snapshot_event(ScreenSnapshotEventInput {
            session_id: session_ref.clone(),
            route: route(),
            title: Some("zellij".to_string()),
            launch: None,
            tab_id: None,
            screen: ScreenSnapshot {
                pane_id,
                sequence: u64::MAX,
                rows: 24,
                cols: 80,
                source: ProjectionSource::ZellijDumpSnapshot,
                buffer_kind: ScreenBufferKind::Unknown,
                surface: ScreenSurface {
                    title: Some("zellij".to_string()),
                    working_directory_uri: None,
                    user_variables: Default::default(),
                    cursor: None,
                    palette: Default::default(),
                    bell_count: 0,
                    progress: Default::default(),
                    lines: vec![ScreenLine::plain("rendered history")],
                },
            },
            buffer_kind: Some("normal".to_string()),
            capture_semantics: Some("rendered_plaintext_snapshot".to_string()),
        })
        .expect("overflowing projection sequence should not fail snapshot persistence");

    let mut connection = store.connection().expect("connection should open");
    let (high_water_event_seq, screen_json) = terminal_screen_snapshots::table
        .filter(terminal_screen_snapshots::id.eq(snapshot_id))
        .select((
            terminal_screen_snapshots::high_water_event_seq,
            terminal_screen_snapshots::screen_json,
        ))
        .first::<(i64, String)>(&mut connection)
        .expect("screen snapshot should load");

    assert_eq!(high_water_event_seq, segment.event_seq_high);
    assert!(screen_json.contains(&u64::MAX.to_string()));
}
