use super::super::super::*;
use super::super::support::*;

#[test]
fn persistence_fault_health_record_is_durable_and_deduped() {
    let store = test_store("persistence-fault-health");
    let (session_id, pane_id, _writer) = session_and_pane(&store);
    let input = PersistenceFaultHealthRecordInput {
        session_id: Some(session_id.clone()),
        pane_id: Some(pane_id.clone()),
        operation: "screen snapshot".to_string(),
        detail: "terminal history persistence failed during screen snapshot - sqlite full"
            .to_string(),
        error_kind: Some("terminal_persistence_v2_error".to_string()),
        metadata: Some(serde_json::json!({ "source": "test" })),
    };

    let first = store
        .record_persistence_fault_health_record(input.clone())
        .expect("fault health record should save");
    let second = store
        .record_persistence_fault_health_record(input)
        .expect("duplicate open fault should dedupe");
    let health =
        store.list_open_data_health_records(Some(&session_id)).expect("health records should list");

    assert_eq!(first, second);
    assert_eq!(health.len(), 1);
    assert_eq!(health[0].pane_id.as_deref(), Some(pane_id.as_str()));
    assert_eq!(health[0].detection_kind, "manual");
    assert_eq!(health[0].severity, "error");
    assert_eq!(health[0].action_state, "open");
    assert!(
        health[0].affected_ref.as_deref().is_some_and(|value| value.contains("persistence_fault"))
    );
}

#[test]
fn integrity_check_flags_unversioned_canonical_json() {
    let store = test_store("unversioned-payload-json");
    let (session_id, pane_id, writer) = session_and_pane(&store);
    let event = store
        .append_journal_event(JournalEventInput {
            session_id: session_id.clone(),
            pane_id: Some(pane_id),
            stream_id: None,
            writer_generation: writer.id,
            event_type: "custom_event".to_string(),
            commit_kind: None,
            payload_json: Some(serde_json::json!({ "custom": true })),
            source_event_id_hash: None,
            occurred_at_ms: None,
            capture_semantics: None,
            trust_level: None,
            metadata: None,
        })
        .expect("custom journal event should persist");

    let mut connection = store.connection().expect("connection should open");
    diesel::update(
        terminal_journal_events::table.filter(terminal_journal_events::id.eq(event.event_id)),
    )
    .set(terminal_journal_events::payload_schema_id.eq(None::<String>))
    .execute(&mut connection)
    .expect("test should remove payload schema id");

    let integrity = store.run_integrity_check().expect("integrity check should run");

    assert_eq!(integrity.result, "failed");
    let error = integrity.error.as_deref().unwrap_or_default();
    assert!(error.contains("history_validation_failures=1"));
    assert!(error.contains("checksum_failures=0"));
    let health = store.list_open_data_health_records(None).expect("health records should list");
    assert_eq!(health.len(), 1);
    assert_eq!(health[0].detection_kind, "migration_mismatch");
    assert_eq!(health[0].severity, "critical");
    assert_eq!(health[0].action_state, "quarantined");
    assert!(
        health[0].affected_ref.as_deref().unwrap_or_default().contains("missing payload_schema_id")
    );
}

#[test]
fn integrity_check_flags_invalid_topology_high_water_json() {
    let store = test_store("topology-high-water-integrity");
    let (session_id, pane_id, writer) = session_and_pane(&store);
    let segment = store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id.clone(),
            writer.id.clone(),
            b"topology high-water target\r\n".to_vec(),
        ))
        .expect("segment should persist");
    let topology_id = store
        .write_topology_snapshot(TopologySnapshotInput {
            id: None,
            session_id,
            writer_generation: writer.id,
            pane_high_water: serde_json::json!({ pane_id.clone(): segment.event_seq_high }),
            topology: serde_json::json!({"tabs":[{"active_pane_id": pane_id}]}),
            source: None,
            metadata: None,
        })
        .expect("topology snapshot should persist");

    let mut connection = store.connection().expect("connection should open");
    diesel::update(
        terminal_topology_snapshots::table.filter(terminal_topology_snapshots::id.eq(&topology_id)),
    )
    .set(terminal_topology_snapshots::pane_high_water_json.eq("[]"))
    .execute(&mut connection)
    .expect("test should corrupt topology high-water json");

    let integrity = store.run_integrity_check().expect("integrity check should run");

    assert_eq!(integrity.result, "failed");
    let error = integrity.error.as_deref().unwrap_or_default();
    assert!(error.contains("history_validation_failures=1"));
    assert!(error.contains("checksum_failures=0"));
    let health = store.list_open_data_health_records(None).expect("health records should list");
    assert_eq!(health.len(), 1);
    assert_eq!(health[0].detection_kind, "projection_drift");
    assert_eq!(health[0].severity, "error");
    assert_eq!(health[0].action_state, "rebuild_pending");
    assert!(health[0].affected_ref.as_deref().unwrap_or_default().contains("pane_high_water_json"));
}

#[test]
fn integrity_check_flags_stream_cursor_drift() {
    let store = test_store("stream-cursor-drift");
    let (session_id, pane_id, writer) = session_and_pane(&store);
    store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id.clone(),
            writer.id,
            b"cursor target\r\n".to_vec(),
        ))
        .expect("segment should persist");

    let mut connection = store.connection().expect("connection should open");
    diesel::update(
        terminal_stream_cursors::table
            .filter(terminal_stream_cursors::pane_id.eq(&pane_id))
            .filter(terminal_stream_cursors::stream_id.eq(DEFAULT_STREAM_ID)),
    )
    .set(terminal_stream_cursors::next_event_seq.eq(99))
    .execute(&mut connection)
    .expect("test should corrupt cursor");

    let integrity = store.run_integrity_check().expect("integrity check should run");

    assert_eq!(integrity.result, "failed");
    let error = integrity.error.as_deref().unwrap_or_default();
    assert!(error.contains("history_validation_failures=1"));
    assert!(error.contains("checksum_failures=0"));
    let health = store.list_open_data_health_records(None).expect("health records should list");
    assert_eq!(health.len(), 1);
    assert_eq!(health[0].detection_kind, "missing_segment");
    assert_eq!(health[0].severity, "error");
    assert_eq!(health[0].action_state, "rebuild_pending");
    assert!(
        health[0]
            .affected_ref
            .as_deref()
            .unwrap_or_default()
            .contains("next_event_seq=99 expected=2")
    );
}

#[test]
fn integrity_check_flags_overlapping_stream_segment_ranges() {
    let store = test_store("stream-overlap");
    let (session_id, pane_id, writer) = session_and_pane(&store);
    let first = store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id.clone(),
            writer.id.clone(),
            b"first\r\n".to_vec(),
        ))
        .expect("first segment should persist");
    let second = store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id,
            pane_id,
            writer.id,
            b"second\r\n".to_vec(),
        ))
        .expect("second segment should persist");

    let mut connection = store.connection().expect("connection should open");
    diesel::update(
        terminal_stream_segments::table.filter(terminal_stream_segments::id.eq(&first.segment_id)),
    )
    .set(terminal_stream_segments::event_seq_high.eq(second.event_seq_low))
    .execute(&mut connection)
    .expect("test should corrupt segment ordering");

    let integrity = store.run_integrity_check().expect("integrity check should run");

    assert_eq!(integrity.result, "failed");
    let error = integrity.error.as_deref().unwrap_or_default();
    assert!(error.contains("history_validation_failures=1"));
    let health = store.list_open_data_health_records(None).expect("health records should list");
    assert_eq!(health.len(), 1);
    assert_eq!(health[0].detection_kind, "missing_segment");
    assert!(health[0].affected_ref.as_deref().unwrap_or_default().contains("overlaps"));
}

#[test]
fn integrity_check_flags_commit_cursor_drift() {
    let store = test_store("commit-cursor-drift");
    let (session_id, pane_id, writer) = session_and_pane(&store);
    store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id,
            writer.id,
            b"commit target\r\n".to_vec(),
        ))
        .expect("segment should persist");

    let mut connection = store.connection().expect("connection should open");
    diesel::update(
        terminal_session_cursors::table
            .filter(terminal_session_cursors::session_id.eq(&session_id)),
    )
    .set(terminal_session_cursors::next_commit_seq.eq(99))
    .execute(&mut connection)
    .expect("test should corrupt session cursor");

    let integrity = store.run_integrity_check().expect("integrity check should run");

    assert_eq!(integrity.result, "failed");
    let error = integrity.error.as_deref().unwrap_or_default();
    assert!(error.contains("history_validation_failures=1"));
    let health = store.list_open_data_health_records(None).expect("health records should list");
    assert_eq!(health.len(), 1);
    assert_eq!(health[0].detection_kind, "missing_segment");
    assert!(
        health[0]
            .affected_ref
            .as_deref()
            .unwrap_or_default()
            .contains("next_commit_seq=99 expected=2")
    );
}

#[test]
fn failed_restore_drill_downgrades_restore_plan() {
    let store = test_store("restore-drill-downgrade");
    let (session_id, pane_id, writer) = session_and_pane(&store);
    let output = store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id.clone(),
            writer.id.clone(),
            b"visible before corruption\r\n".to_vec(),
        ))
        .expect("segment should persist");
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
            screen: serde_json::json!({"lines":["visible before corruption"]}),
            parser_version: None,
            projection_version: None,
            metadata: None,
        })
        .expect("screen snapshot should persist");
    let mut connection = store.connection().expect("connection should open");
    diesel::update(terminal_stream_segments::table)
        .filter(terminal_stream_segments::id.eq(&output.segment_id))
        .set(terminal_stream_segments::checksum.eq("not-the-real-checksum"))
        .execute(&mut connection)
        .expect("test should corrupt checksum");

    let drill = store.run_restore_drill(&session_id).expect("restore drill should run");
    let plan = store.restore_plan(&session_id).expect("restore plan should reload");

    assert_eq!(drill.result, "failed");
    assert_eq!(plan.guarantee_level, RestoreGuaranteeLevel::DegradedHistory);
    assert_eq!(plan.latest_restore_drill_status.as_deref(), Some("failed"));
    assert!(plan.evidence.iter().any(|evidence| {
        evidence.kind == "latest_restore_drill_status" && evidence.value == "failed"
    }));
}
