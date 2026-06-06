use super::super::super::*;
use super::super::support::*;

#[test]
fn appends_raw_stream_segments_and_replays_after_reopen() {
    let store = test_store("stream");
    let path = store.path().to_path_buf();
    let (session_id, pane_id, writer) = session_and_pane(&store);

    let first = store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id.clone(),
            writer.id.clone(),
            b"git status\r\n".to_vec(),
        ))
        .expect("first segment should persist");
    let second = store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id.clone(),
            writer.id,
            b"fatal: not a git repository\r\n".to_vec(),
        ))
        .expect("second segment should persist");

    assert_eq!(first.event_seq_low, 1);
    assert_eq!(second.event_seq_low, 2);
    assert_eq!(second.byte_low, first.byte_high);

    let reopened =
        TerminalPersistenceV2::open_with_config(path, TerminalPersistenceV2Config::test())
            .expect("store should reopen");
    let segments =
        reopened.list_stream_segments(&session_id, &pane_id, 1, 10).expect("segments should read");
    let payload: Vec<u8> = segments.into_iter().flat_map(|segment| segment.payload).collect();

    assert_eq!(payload, b"git status\r\nfatal: not a git repository\r\n");

    let hydrated = reopened
        .hydrate_pane_history(&session_id, &pane_id, Some(1), Some(10), Some(1024))
        .expect("pane history should hydrate");

    assert_eq!(hydrated.segments.len(), 2);
    assert_eq!(hydrated.gaps.len(), 0);
    assert_eq!(hydrated.replay_strategy, PaneHistoryReplayStrategy::RawVtStream);
    assert_eq!(
        hydrated.segments.iter().flat_map(|segment| segment.payload.clone()).collect::<Vec<_>>(),
        b"git status\r\nfatal: not a git repository\r\n"
    );
}

#[test]
fn raw_stream_persists_alternate_screen_events_without_replaying_tui_as_scrollback() {
    let store = test_store("alternate-screen-events");
    let (session_id, pane_id, writer) = session_and_pane(&store);

    let tui = store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id.clone(),
            writer.id.clone(),
            b"before\x1b[?1049hinside tui\x1b[?1049lafter\r\n".to_vec(),
        ))
        .expect("tui segment should persist");
    let after = store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id.clone(),
            writer.id,
            b"shell again\r\n".to_vec(),
        ))
        .expect("post-tui segment should persist after derived mode events");

    assert_eq!(tui.event_seq_low, 1);
    assert_eq!(tui.event_seq_high, 1);
    assert_eq!(after.event_seq_low, 4);

    let mut connection = store.connection().expect("connection should open");
    let events = terminal_journal_events::table
        .filter(terminal_journal_events::session_id.eq(&session_id))
        .filter(terminal_journal_events::pane_id.eq(Some(pane_id.clone())))
        .order(terminal_journal_events::event_seq.asc())
        .select((
            terminal_journal_events::event_type,
            terminal_journal_events::event_seq,
            terminal_journal_events::payload_json,
            terminal_journal_events::byte_low,
            terminal_journal_events::byte_high,
        ))
        .load::<(String, i64, Option<String>, Option<i64>, Option<i64>)>(&mut connection)
        .expect("journal events should load");

    assert_eq!(events.len(), 4);
    assert_eq!(events[0].0, "terminal_output");
    assert_eq!(events[1].0, "terminal_buffer_mode");
    assert_eq!(events[1].1, 2);
    assert_eq!(events[2].0, "terminal_buffer_mode");
    assert_eq!(events[2].1, 3);
    assert_eq!(events[3].0, "terminal_output");
    assert_eq!(events[3].1, 4);
    assert!(events[1].3.expect("enter byte_low should exist") >= tui.byte_low);
    assert!(events[1].4.expect("enter byte_high should exist") <= tui.byte_high);

    let enter: Value =
        serde_json::from_str(events[1].2.as_deref().expect("enter payload should be persisted"))
            .expect("enter payload should be json");
    let leave: Value =
        serde_json::from_str(events[2].2.as_deref().expect("leave payload should be persisted"))
            .expect("leave payload should be json");
    assert_eq!(enter["action"], "enter");
    assert_eq!(enter["target_buffer_kind"], "alternate");
    assert_eq!(leave["action"], "leave");
    assert_eq!(leave["target_buffer_kind"], "normal");

    let cursor_next = terminal_stream_cursors::table
        .filter(terminal_stream_cursors::session_id.eq(&session_id))
        .filter(terminal_stream_cursors::pane_id.eq(&pane_id))
        .select(terminal_stream_cursors::next_event_seq)
        .first::<i64>(&mut connection)
        .expect("stream cursor should load");
    assert_eq!(cursor_next, 5);

    let integrity = store.run_integrity_check().expect("integrity check should pass");
    assert_eq!(integrity.result, "passed");
}

#[test]
fn hydrate_pane_history_respects_byte_budget_for_long_output() {
    let store = test_store("long-output-budget");
    let (session_id, pane_id, writer) = session_and_pane(&store);
    let first_payload = vec![b'a'; 400];
    let second_payload = vec![b'b'; 400];
    let third_payload = vec![b'c'; 120];
    store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id.clone(),
            writer.id.clone(),
            first_payload.clone(),
        ))
        .expect("first segment should persist");
    store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id.clone(),
            writer.id.clone(),
            second_payload.clone(),
        ))
        .expect("second segment should persist");
    store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id.clone(),
            writer.id.clone(),
            third_payload.clone(),
        ))
        .expect("third segment should persist");
    store.release_writer_generation(&writer.id).expect("writer should release");

    let first_page = store
        .hydrate_pane_history(&session_id, &pane_id, Some(1), Some(10), Some(700))
        .expect("first history page should hydrate");
    assert_eq!(first_page.segments.len(), 1);
    assert_eq!(first_page.segments[0].payload, first_payload);
    assert_eq!(first_page.total_payload_bytes, 400);
    assert_eq!(first_page.next_event_seq, Some(2));
    assert!(first_page.has_more_segments);

    let second_page = store
        .hydrate_pane_history(
            &session_id,
            &pane_id,
            first_page.next_event_seq,
            Some(10),
            Some(1_000),
        )
        .expect("second history page should hydrate");
    assert_eq!(second_page.segments.len(), 2);
    assert_eq!(second_page.segments[0].payload, second_payload);
    assert_eq!(second_page.segments[1].payload, third_payload);
    assert_eq!(second_page.total_payload_bytes, 520);
    assert_eq!(second_page.next_event_seq, Some(4));
    assert!(!second_page.has_more_segments);
}

#[test]
fn hydrate_pane_history_filters_gaps_to_requested_page() {
    let store = test_store("history-gap-page-window");
    let (session_id, pane_id, writer) = session_and_pane(&store);
    let first = store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id.clone(),
            writer.id.clone(),
            b"before gap\r\n".to_vec(),
        ))
        .expect("first segment should persist");
    store
        .append_history_gap_event(
            &session_id,
            &pane_id,
            &writer.id,
            2,
            Some(128),
            "test receiver lag",
            Some(42),
        )
        .expect("gap should persist");
    let after_gap = store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id.clone(),
            writer.id.clone(),
            b"after gap\r\n".to_vec(),
        ))
        .expect("post-gap segment should persist");
    store.release_writer_generation(&writer.id).expect("writer should release");

    assert_eq!(first.event_seq_low, 1);
    assert_eq!(after_gap.event_seq_low, 4);

    let first_page = store
        .hydrate_pane_history(&session_id, &pane_id, Some(1), Some(1), Some(1024))
        .expect("first history page should hydrate");
    assert_eq!(first_page.segments.len(), 1);
    assert_eq!(first_page.segments[0].id, first.segment_id);
    assert!(first_page.gaps.is_empty());
    assert_eq!(first_page.next_event_seq, Some(2));
    assert!(first_page.has_more_segments);

    let second_page = store
        .hydrate_pane_history(&session_id, &pane_id, first_page.next_event_seq, Some(1), Some(1024))
        .expect("second history page should hydrate");
    assert_eq!(second_page.segments.len(), 1);
    assert_eq!(second_page.segments[0].id, after_gap.segment_id);
    assert_eq!(second_page.gaps.len(), 1);
    assert_eq!(second_page.gaps[0].event_seq_low, Some(2));
    assert_eq!(second_page.gaps[0].event_seq_high, Some(3));
    assert_eq!(second_page.next_event_seq, Some(5));
    assert!(!second_page.has_more_segments);
}

#[test]
fn hydrate_pane_history_keeps_cursor_for_trailing_gap_page() {
    let store = test_store("history-trailing-gap-page");
    let (session_id, pane_id, writer) = session_and_pane(&store);
    let first = store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id.clone(),
            writer.id.clone(),
            b"before trailing gap\r\n".to_vec(),
        ))
        .expect("first segment should persist");
    store
        .append_history_gap_event(
            &session_id,
            &pane_id,
            &writer.id,
            2,
            Some(128),
            "test trailing receiver lag",
            Some(42),
        )
        .expect("trailing gap should persist");
    store.release_writer_generation(&writer.id).expect("writer should release");

    let first_page = store
        .hydrate_pane_history(&session_id, &pane_id, Some(1), Some(1), Some(1024))
        .expect("first history page should hydrate");
    assert_eq!(first_page.segments.len(), 1);
    assert_eq!(first_page.segments[0].id, first.segment_id);
    assert!(first_page.gaps.is_empty());
    assert_eq!(first_page.next_event_seq, Some(2));
    assert!(first_page.has_more_segments);

    let second_page = store
        .hydrate_pane_history(&session_id, &pane_id, first_page.next_event_seq, Some(1), Some(1024))
        .expect("trailing gap page should hydrate");
    assert!(second_page.segments.is_empty());
    assert_eq!(second_page.gaps.len(), 1);
    assert_eq!(second_page.gaps[0].event_seq_low, Some(2));
    assert_eq!(second_page.gaps[0].event_seq_high, Some(3));
    assert_eq!(second_page.next_event_seq, Some(4));
    assert!(!second_page.has_more_segments);
}

#[test]
fn hydrate_pane_history_filters_gaps_before_limit_for_late_pages() {
    let store = test_store("history-gap-prefilter-before-limit");
    let (session_id, pane_id, writer) = session_and_pane(&store);
    let target_event_seq = MAX_HISTORY_GAP_LIMIT + 2;

    for index in 0..(MAX_HISTORY_GAP_LIMIT + 5) {
        store
            .append_history_gap_event(
                &session_id,
                &pane_id,
                &writer.id,
                1,
                Some(index),
                &format!("test gap {index}"),
                Some(index),
            )
            .expect("gap should persist");
    }
    store.release_writer_generation(&writer.id).expect("writer should release");

    let late_page = store
        .hydrate_pane_history(&session_id, &pane_id, Some(target_event_seq), Some(1), Some(1024))
        .expect("late gap page should hydrate");

    assert!(late_page.segments.is_empty());
    assert!(late_page.gaps.iter().any(|gap| {
        gap.event_seq_low == Some(target_event_seq)
            && gap.reason == format!("test gap {}", target_event_seq - 1)
    }));
}

#[test]
fn hydrate_pane_history_pages_gap_only_history() {
    let store = test_store("history-gap-only-pagination");
    let (session_id, pane_id, writer) = session_and_pane(&store);

    for index in 0..(MAX_HISTORY_GAP_LIMIT + 2) {
        store
            .append_history_gap_event(
                &session_id,
                &pane_id,
                &writer.id,
                1,
                Some(index),
                &format!("gap-only {index}"),
                Some(index),
            )
            .expect("gap should persist");
    }
    store.release_writer_generation(&writer.id).expect("writer should release");

    let first_page = store
        .hydrate_pane_history(&session_id, &pane_id, Some(1), Some(10), Some(1024))
        .expect("first gap-only page should hydrate");
    assert!(first_page.segments.is_empty());
    assert_eq!(first_page.gaps.len(), MAX_HISTORY_GAP_LIMIT as usize);
    assert_eq!(first_page.next_event_seq, Some(MAX_HISTORY_GAP_LIMIT + 1));
    assert!(first_page.has_more_segments);

    let second_page = store
        .hydrate_pane_history(
            &session_id,
            &pane_id,
            first_page.next_event_seq,
            Some(10),
            Some(1024),
        )
        .expect("second gap-only page should hydrate");
    assert!(second_page.segments.is_empty());
    assert_eq!(second_page.gaps.len(), 2);
    assert_eq!(second_page.gaps[0].event_seq_low, Some(MAX_HISTORY_GAP_LIMIT + 1));
    assert_eq!(second_page.next_event_seq, Some(MAX_HISTORY_GAP_LIMIT + 3));
    assert!(!second_page.has_more_segments);
}

#[test]
fn hydrate_pane_history_advances_through_corrupt_segment_when_page_is_small() {
    let store = test_store("history-corrupt-page-window");
    let (session_id, pane_id, writer) = session_and_pane(&store);
    let corrupt = store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id.clone(),
            writer.id.clone(),
            b"corrupt me\r\n".to_vec(),
        ))
        .expect("corrupt candidate should persist");
    let healthy = store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id.clone(),
            writer.id.clone(),
            b"still visible\r\n".to_vec(),
        ))
        .expect("healthy segment should persist");
    store.release_writer_generation(&writer.id).expect("writer should release");
    let mut connection = store.connection().expect("connection should open");
    diesel::update(
        terminal_stream_segments::table
            .filter(terminal_stream_segments::id.eq(&corrupt.segment_id)),
    )
    .set(terminal_stream_segments::checksum.eq("not-the-real-checksum"))
    .execute(&mut connection)
    .expect("test should corrupt checksum");

    let hydrated = store
        .hydrate_pane_history(&session_id, &pane_id, Some(1), Some(1), Some(1024))
        .expect("hydration should skip corrupt bytes and keep paging usable");

    assert_eq!(hydrated.segments.len(), 1);
    assert_eq!(hydrated.segments[0].id, healthy.segment_id);
    assert_eq!(hydrated.next_event_seq, Some(3));
    assert!(!hydrated.has_more_segments);
    assert!(hydrated.gaps.iter().any(|gap| {
        gap.gap_kind == "corrupted_segment"
            && gap.event_seq_low == Some(corrupt.event_seq_low)
            && gap.event_seq_high == Some(corrupt.event_seq_high)
    }));
}

#[test]
fn legacy_visual_snapshot_import_preserves_raw_stream_cursor() {
    let store = test_store("visual-import-preserves-cursor");
    let (session_id, pane_id, writer) = session_and_pane(&store);
    let first = store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id.clone(),
            writer.id.clone(),
            b"cmd one\r\n".to_vec(),
        ))
        .expect("first segment should persist");
    let second = store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id.clone(),
            writer.id.clone(),
            b"cmd two\r\n".to_vec(),
        ))
        .expect("second segment should persist");
    let third = store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id.clone(),
            writer.id.clone(),
            b"cmd three\r\n".to_vec(),
        ))
        .expect("third segment should persist");
    store.release_writer_generation(&writer.id).expect("writer should release");

    assert_eq!(first.event_seq_low, 1);
    assert_eq!(second.event_seq_low, 2);
    assert_eq!(third.event_seq_low, 3);

    let session_uuid = Uuid::parse_str(&session_id).expect("session id should be uuid");
    let pane_uuid = Uuid::parse_str(&pane_id).expect("pane id should be uuid");
    let session_typed = SessionId(session_uuid);
    let pane_typed = PaneId(pane_uuid);
    let tab_id = TabId::new();
    let saved = SavedNativeSession {
        session_id: session_typed,
        route: route(),
        title: Some("visual import should not rewrite raw cursor".to_string()),
        launch: None,
        manifest: SavedSessionManifest::current(),
        topology: TopologySnapshot {
            session_id: session_typed,
            backend_kind: BackendKind::Native,
            tabs: vec![TabSnapshot {
                tab_id,
                title: Some("main".to_string()),
                root: PaneTreeNode::Leaf { pane_id: pane_typed },
                focused_pane: Some(pane_typed),
            }],
            focused_tab: Some(tab_id),
        },
        screens: vec![ScreenSnapshot {
            pane_id: pane_typed,
            sequence: 6,
            rows: 24,
            cols: 80,
            source: ProjectionSource::NativeEmulator,
            surface: ScreenSurface {
                title: Some("visual import should not rewrite raw cursor".to_string()),
                cursor: None,
                lines: vec![ScreenLine {
                    text: "visual snapshot sequence is not event sequence".to_string(),
                }],
            },
        }],
        saved_at_ms: 1_700_000_000_000,
    };

    store
        .import_saved_native_session_snapshot(&saved)
        .expect("legacy visual snapshot should import");

    let mut connection = store.connection().expect("connection should open");
    let cursor = load_stream_cursor(&mut connection, &session_id, &pane_id, DEFAULT_STREAM_ID)
        .expect("stream cursor should load");
    let pane_last_event_seq = terminal_panes::table
        .filter(terminal_panes::id.eq(&pane_id))
        .select(terminal_panes::last_event_seq)
        .first::<i64>(&mut connection)
        .expect("pane cursor should load");

    assert_eq!(cursor.next_event_seq, 4);
    assert_eq!(cursor.next_byte_seq, third.byte_high);
    assert_eq!(pane_last_event_seq, 3);

    let drill = store.run_restore_drill(&session_id).expect("restore drill should run");
    assert_eq!(drill.result, "passed");
}
