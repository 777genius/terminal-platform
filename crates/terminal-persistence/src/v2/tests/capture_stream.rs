use super::super::*;
use super::support::*;

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

#[test]
fn rejects_unknown_capture_semantics_before_stream_insert() {
    let store = test_store("capture-semantics-domain");
    let (session_id, pane_id, writer) = session_and_pane(&store);
    let mut input = StreamSegmentInput::terminal_output(
        session_id.clone(),
        pane_id.clone(),
        writer.id,
        b"rendered text\r\n".to_vec(),
    );
    input.capture_semantics = Some("probably_plain_text".to_string());

    let error = store
        .append_stream_segment(input)
        .expect_err("unknown capture semantics should fail before insert");
    let mut connection = store.connection().expect("connection should open");
    let segment_count = terminal_stream_segments::table
        .filter(terminal_stream_segments::session_id.eq(&session_id))
        .count()
        .get_result::<i64>(&mut connection)
        .expect("segment count should load");

    assert!(
        matches!(error, TerminalPersistenceV2Error::InvalidData(message) if message.contains("unknown capture semantics"))
    );
    assert_eq!(segment_count, 0);
}

#[test]
fn rejects_unknown_backend_capability_domains_before_insert() {
    let store = test_store("backend-capability-api-domain");
    let session_id = Uuid::new_v4().to_string();
    let id = "invalid-backend-capability-api-domain".to_string();

    let error = store
        .record_backend_capability_report(BackendCapabilityReportInput {
            id: Some(id.clone()),
            session_id: Some(session_id),
            backend_kind: "native".to_string(),
            backend_version: Some("test".to_string()),
            backend_binary_path_hash: Some("test-path-hash".to_string()),
            route_kind: "local_daemon".to_string(),
            probe_status: "passed".to_string(),
            capture_strategy: "rawish_stream".to_string(),
            capture_semantics: "raw_vt_stream".to_string(),
            can_preserve_process_when_live: false,
            can_capture_scrollback: true,
            command_boundary_confidence: "high".to_string(),
            evidence: None,
            expires_at_ms: None,
        })
        .expect_err("unknown capture strategy should fail before insert");
    let mut connection = store.connection().expect("connection should open");
    let capability_count = terminal_backend_capability_reports::table
        .filter(terminal_backend_capability_reports::id.eq(&id))
        .count()
        .get_result::<i64>(&mut connection)
        .expect("capability count should load");

    assert!(
        matches!(error, TerminalPersistenceV2Error::InvalidData(message) if message.contains("unknown capture strategy"))
    );
    assert_eq!(capability_count, 0);

    let probe_id = "invalid-backend-probe-status-api-domain".to_string();
    let probe_error = store
        .record_backend_capability_report(BackendCapabilityReportInput {
            id: Some(probe_id.clone()),
            session_id: Some(Uuid::new_v4().to_string()),
            backend_kind: "native".to_string(),
            backend_version: Some("test".to_string()),
            backend_binary_path_hash: Some("test-path-hash".to_string()),
            route_kind: "local_daemon".to_string(),
            probe_status: "maybe".to_string(),
            capture_strategy: "raw_stream".to_string(),
            capture_semantics: "raw_vt_stream".to_string(),
            can_preserve_process_when_live: false,
            can_capture_scrollback: true,
            command_boundary_confidence: "high".to_string(),
            evidence: None,
            expires_at_ms: None,
        })
        .expect_err("unknown probe status should fail before insert");
    let probe_count = terminal_backend_capability_reports::table
        .filter(terminal_backend_capability_reports::id.eq(&probe_id))
        .count()
        .get_result::<i64>(&mut connection)
        .expect("probe capability count should load");

    assert!(
        matches!(probe_error, TerminalPersistenceV2Error::InvalidData(message) if message.contains("unknown backend probe status"))
    );
    assert_eq!(probe_count, 0);
}

#[test]
fn backend_capability_mapper_outputs_db_valid_domains() {
    let store = test_store("backend-capability-mapper-domains");

    let unknown = BackendCapabilityReportInput::from_backend_capabilities(
        BackendKind::Zellij,
        "imported_foreign",
        &BackendCapabilities::default(),
    );
    assert_eq!(unknown.capture_strategy, "unknown");
    assert_eq!(unknown.capture_semantics, "rendered_plaintext_snapshot");
    store
        .record_backend_capability_report(unknown)
        .expect("unknown strategy is a valid conservative capability report");

    let mut snapshot_capabilities = BackendCapabilities::default();
    snapshot_capabilities.rendered_scrollback_snapshot = true;
    let snapshot = BackendCapabilityReportInput::from_backend_capabilities(
        BackendKind::Tmux,
        "imported_foreign",
        &snapshot_capabilities,
    );
    assert_eq!(snapshot.capture_strategy, "rendered_snapshot");
    assert_eq!(snapshot.capture_semantics, "rendered_plaintext_snapshot");
    store
        .record_backend_capability_report(snapshot)
        .expect("rendered snapshot strategy should persist");

    let mut raw_capabilities = BackendCapabilities::default();
    raw_capabilities.raw_output_stream = true;
    let raw = BackendCapabilityReportInput::from_backend_capabilities(
        BackendKind::Native,
        "local_daemon",
        &raw_capabilities,
    );
    assert_eq!(raw.capture_strategy, "raw_stream");
    assert_eq!(raw.capture_semantics, "raw_vt_stream");
    store.record_backend_capability_report(raw).expect("raw strategy should persist");
}

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

#[test]
fn records_delivery_offsets_and_builds_replay_window() {
    let store = test_store("delivery-offset");
    let (session_id, pane_id, writer) = session_and_pane(&store);
    store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id.clone(),
            writer.id.clone(),
            b"first\r\n".to_vec(),
        ))
        .expect("first segment should persist");
    store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id.clone(),
            writer.id,
            b"second\r\n".to_vec(),
        ))
        .expect("second segment should persist");
    let client = store
        .upsert_delivery_client(DeliveryClientInput {
            id: Some("browser-a".to_string()),
            client_kind: "browser".to_string(),
            install_ref_hash: None,
            browser_profile_ref_hash: None,
            user_agent_hash: None,
            trust_state: None,
        })
        .expect("client should persist");

    let sent = store
        .record_delivery_progress(DeliveryProgressInput {
            client_id: client.id.clone(),
            session_id: session_id.clone(),
            pane_id: pane_id.clone(),
            stream_id: None,
            last_sent_event_seq: Some(2),
            last_acked_event_seq: None,
        })
        .expect("sent offset should persist");
    let acked = store
        .record_delivery_progress(DeliveryProgressInput {
            client_id: client.id.clone(),
            session_id: session_id.clone(),
            pane_id: pane_id.clone(),
            stream_id: None,
            last_sent_event_seq: None,
            last_acked_event_seq: Some(1),
        })
        .expect("acked offset should persist");
    let window = store
        .delivery_replay_window(DeliveryOffsetInput {
            client_id: client.id.clone(),
            session_id: session_id.clone(),
            pane_id: pane_id.clone(),
            stream_id: None,
        })
        .expect("replay window should load");
    let replay = store
        .hydrate_pane_history(&session_id, &pane_id, window.from_event_seq, Some(10), Some(1024))
        .expect("replay history should hydrate");

    assert_eq!(sent.last_sent_event_seq, 2);
    assert_eq!(acked.last_acked_event_seq, 1);
    assert_eq!(acked.replay_from_event_seq, Some(2));
    assert_eq!(window.from_event_seq, Some(2));
    assert_eq!(window.to_event_seq, 2);
    assert_eq!(window.gap_state, "none");
    assert_eq!(replay.segments.len(), 1);
    assert_eq!(replay.segments[0].payload, b"second\r\n");

    let fully_acked = store
        .record_delivery_progress(DeliveryProgressInput {
            client_id: client.id.clone(),
            session_id: session_id.clone(),
            pane_id: pane_id.clone(),
            stream_id: None,
            last_sent_event_seq: None,
            last_acked_event_seq: Some(2),
        })
        .expect("fully acked offset should persist");
    let empty_window = store
        .delivery_replay_window(DeliveryOffsetInput {
            client_id: client.id,
            session_id,
            pane_id,
            stream_id: None,
        })
        .expect("empty replay window should load");

    assert_eq!(fully_acked.replay_from_event_seq, None);
    assert_eq!(empty_window.from_event_seq, None);
    assert_eq!(empty_window.to_event_seq, 2);
}

#[test]
fn delivery_replay_window_surfaces_gap_state() {
    let store = test_store("delivery-gap");
    let (session_id, pane_id, writer) = session_and_pane(&store);
    store.release_writer_generation(&writer.id).expect("writer should release");
    store
        .record_history_gap_event(HistoryGapEventInput {
            session_id: session_id.clone(),
            route: route(),
            title: Some("shell".to_string()),
            launch: None,
            pane_id: pane_id.clone(),
            tab_id: None,
            rows: Some(24),
            cols: Some(80),
            skipped_events: 2,
            estimated_dropped_bytes: Some(64),
            reason: "test_delivery_gap".to_string(),
            occurred_at_ms: None,
        })
        .expect("history gap should persist");
    let client = store
        .upsert_delivery_client(DeliveryClientInput {
            id: Some("browser-gap".to_string()),
            client_kind: "browser".to_string(),
            install_ref_hash: None,
            browser_profile_ref_hash: None,
            user_agent_hash: None,
            trust_state: None,
        })
        .expect("client should persist");

    let window = store
        .delivery_replay_window(DeliveryOffsetInput {
            client_id: client.id,
            session_id,
            pane_id,
            stream_id: None,
        })
        .expect("replay window should load");

    assert_eq!(window.from_event_seq, Some(1));
    assert_eq!(window.to_event_seq, 2);
    assert_eq!(window.gap_state, "gap");
}

#[test]
fn stream_segment_enqueue_projection_outbox_message() {
    let store = test_store("outbox-stream");
    let (session_id, pane_id, writer) = session_and_pane(&store);
    let receipt = store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id.clone(),
            writer.id,
            b"outbox\r\n".to_vec(),
        ))
        .expect("stream segment should persist");

    let message = store
        .claim_next_outbox_message("projection-worker", 60_000)
        .expect("claim should load")
        .expect("projection outbox message should exist");

    assert_eq!(message.message_kind, "pane_history_projection");
    assert_eq!(message.state, "claimed");
    assert_eq!(message.attempts, 1);
    assert_eq!(message.payload_json["session_id"], session_id);
    assert_eq!(message.payload_json["pane_id"], pane_id);
    assert_eq!(message.payload_json["commit_id"], receipt.commit_id);
}

#[test]
fn outbox_dedupes_claims_and_completes_by_lease_token() {
    let store = test_store("outbox-dedupe");
    let first = store
        .enqueue_outbox_message(OutboxMessageInput {
            message_kind: "restore_drill".to_string(),
            payload: serde_json::json!({ "session_id": "session-a" }),
            dedupe_key: Some("restore-drill:session-a".to_string()),
            max_attempts: None,
            next_run_at_ms: None,
        })
        .expect("first outbox message should enqueue");
    let second = store
        .enqueue_outbox_message(OutboxMessageInput {
            message_kind: "restore_drill".to_string(),
            payload: serde_json::json!({ "session_id": "session-a" }),
            dedupe_key: Some("restore-drill:session-a".to_string()),
            max_attempts: None,
            next_run_at_ms: None,
        })
        .expect("deduped outbox message should load");

    let claim = store
        .claim_next_outbox_message("worker-a", 60_000)
        .expect("claim should succeed")
        .expect("message should be claimable");
    let second_claim =
        store.claim_next_outbox_message("worker-b", 60_000).expect("second claim should not fail");
    let wrong_token_done = store
        .mark_outbox_message_done(&claim.id, "wrong-token")
        .expect("wrong token completion should be safe");
    let done = store
        .mark_outbox_message_done(
            &claim.id,
            claim.lease_token.as_deref().expect("claim should have a lease token"),
        )
        .expect("completion should succeed");
    let no_more = store
        .claim_next_outbox_message("worker-a", 60_000)
        .expect("done message should not be claimable");

    assert_eq!(first.id, second.id);
    assert_eq!(claim.id, first.id);
    assert!(second_claim.is_none());
    assert!(!wrong_token_done);
    assert!(done);
    assert!(no_more.is_none());
}

#[test]
fn outbox_quarantines_poison_message_after_max_attempts() {
    let store = test_store("outbox-quarantine");
    let message = store
        .enqueue_outbox_message(OutboxMessageInput {
            message_kind: "integrity_check".to_string(),
            payload: serde_json::json!({ "scope": "test" }),
            dedupe_key: None,
            max_attempts: Some(1),
            next_run_at_ms: None,
        })
        .expect("message should enqueue");
    let claim = store
        .claim_next_outbox_message("worker-a", 60_000)
        .expect("claim should succeed")
        .expect("message should be claimable");

    let failed = store
        .fail_outbox_message(
            &message.id,
            claim.lease_token.as_deref().expect("claim should have a lease token"),
            "synthetic failure",
        )
        .expect("failure should persist");
    let no_more = store
        .claim_next_outbox_message("worker-b", 60_000)
        .expect("quarantined message should not be claimable");

    assert_eq!(failed.state, "quarantined");
    assert_eq!(failed.last_error.as_deref(), Some("synthetic failure"));
    assert!(no_more.is_none());
}
