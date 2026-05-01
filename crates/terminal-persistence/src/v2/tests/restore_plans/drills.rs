use super::super::super::*;
use super::super::support::*;

#[test]
fn runs_integrity_check_and_restore_drill() {
    let store = test_store("integrity-drill");
    let (session_id, pane_id, writer) = session_and_pane(&store);
    let output = store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id.clone(),
            writer.id.clone(),
            b"durable history\r\n".to_vec(),
        ))
        .expect("segment should persist");
    store
        .write_screen_snapshot(ScreenSnapshotInput {
            id: None,
            session_id: session_id.clone(),
            pane_id: pane_id.clone(),
            writer_generation: writer.id.clone(),
            projection_source: None,
            buffer_kind: None,
            rows: 24,
            cols: 80,
            base_event_seq: output.event_seq_low,
            high_water_event_seq: output.event_seq_high,
            high_water_byte_seq: Some(output.byte_high),
            screen: serde_json::json!({"lines":["durable history"]}),
            parser_version: None,
            projection_version: None,
            metadata: None,
        })
        .expect("screen snapshot should persist");

    let integrity = store.run_integrity_check().expect("integrity check should run");
    let drill = store.run_restore_drill(&session_id).expect("restore drill should run");

    assert_eq!(integrity.result, "passed");
    assert_eq!(drill.result, "passed");
    assert_eq!(drill.restore_guarantee_level, "basic_history");
    assert!(drill.error.is_none());

    let plan = store.restore_plan(&session_id).expect("restore plan should reload");
    assert_eq!(plan.latest_restore_drill_status.as_deref(), Some("passed"));
}

#[test]
fn restore_drill_records_replay_sandbox_side_effect_evidence() {
    let store = test_store("restore-replay-sandbox");
    let (session_id, pane_id, writer) = session_and_pane(&store);
    let payload = b"\x1b]52;c;Zm9v\x07\x1b]0;owned-title\x07\x1b]8;;https://example.test\x07link\x1b]8;;\x07\x1b]7;file://C:/repo\x07\x1b]133;A\x07bell\x07"
            .to_vec();
    let output = store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id.clone(),
            writer.id.clone(),
            payload,
        ))
        .expect("control-sequence segment should persist");
    store
        .write_screen_snapshot(ScreenSnapshotInput {
            id: None,
            session_id: session_id.clone(),
            pane_id,
            writer_generation: writer.id,
            projection_source: None,
            buffer_kind: None,
            rows: 24,
            cols: 80,
            base_event_seq: output.event_seq_low,
            high_water_event_seq: output.event_seq_high,
            high_water_byte_seq: Some(output.byte_high),
            screen: serde_json::json!({"lines":["link"]}),
            parser_version: None,
            projection_version: None,
            metadata: None,
        })
        .expect("screen snapshot should persist");

    let safety = store
        .restore_replay_safety_diagnostics(&session_id)
        .expect("replay safety diagnostics should load");
    let drill = store.run_restore_drill(&session_id).expect("restore drill should run");

    assert_eq!(drill.result, "passed");
    assert!(safety.side_effects_suppressed);
    assert!(safety.prompt_injection_text_is_data);
    assert_eq!(safety.osc52_clipboard_count, 1);
    assert_eq!(safety.title_sequence_count, 1);
    assert_eq!(safety.hyperlink_sequence_count, 2);
    assert_eq!(safety.cwd_sequence_count, 1);
    assert_eq!(safety.shell_marker_sequence_count, 1);

    let mut connection = store.connection().expect("connection should open");
    let evidence_json = terminal_restore_drills::table
        .filter(terminal_restore_drills::id.eq(&drill.id))
        .select(terminal_restore_drills::evidence_json)
        .first::<Option<String>>(&mut connection)
        .expect("restore drill evidence should load")
        .expect("restore drill evidence should exist");
    assert!(evidence_json.contains("historical_replay_side_effects_suppressed"));
    assert!(evidence_json.contains("historical_replay_osc52_clipboard_count"));
    assert!(evidence_json.contains("historical_replay_prompt_injection_text_is_data"));
}
