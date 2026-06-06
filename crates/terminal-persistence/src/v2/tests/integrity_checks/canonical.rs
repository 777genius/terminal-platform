use super::super::super::*;
use super::super::support::*;

#[test]
fn canonical_json_payloads_are_versioned() {
    let store = test_store("payload-schema-contracts");
    let (session_id, pane_id, writer) = session_and_pane(&store);

    store
        .append_ui_input_event_and_command(
            &UiInputEventInput {
                session_id: session_id.clone(),
                route: route(),
                title: None,
                launch: None,
                pane_id: pane_id.clone(),
                data: "git status\r".to_string(),
                is_paste: false,
                source_event_id: None,
                rows: Some(24),
                cols: Some(80),
                shell_kind: Some("cmd".to_string()),
            },
            &writer.id,
        )
        .expect("ui input event should persist");
    store
        .append_history_gap_event(
            &session_id,
            &pane_id,
            &writer.id,
            2,
            Some(12),
            "queue_pressure",
            None,
        )
        .expect("history gap should persist");
    store
        .append_journal_event(JournalEventInput {
            session_id: session_id.clone(),
            pane_id: Some(pane_id.clone()),
            stream_id: None,
            writer_generation: writer.id.clone(),
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
    store
        .write_topology_snapshot(TopologySnapshotInput {
            id: None,
            session_id: session_id.clone(),
            writer_generation: writer.id,
            pane_high_water: serde_json::json!({ pane_id.clone(): 4 }),
            topology: serde_json::json!({ "tabs": [] }),
            source: None,
            metadata: None,
        })
        .expect("topology snapshot should persist");

    let mut connection = store.connection().expect("connection should open");
    let events = terminal_journal_events::table
        .filter(terminal_journal_events::session_id.eq(&session_id))
        .select((terminal_journal_events::event_type, terminal_journal_events::payload_schema_id))
        .load::<(String, Option<String>)>(&mut connection)
        .expect("journal schema ids should load");
    assert!(events.iter().any(|(event_type, schema_id)| {
        event_type == "terminal_input" && schema_id.as_deref() == Some(PAYLOAD_SCHEMA_UI_INPUT_V1)
    }));
    assert!(events.iter().any(|(event_type, schema_id)| {
        event_type == "history_gap" && schema_id.as_deref() == Some(PAYLOAD_SCHEMA_HISTORY_GAP_V1)
    }));
    assert!(events.iter().any(|(event_type, schema_id)| {
        event_type == "custom_event"
            && schema_id.as_deref() == Some(PAYLOAD_SCHEMA_JOURNAL_EVENT_V1)
    }));

    let topology_schema_id = terminal_topology_snapshots::table
        .filter(terminal_topology_snapshots::session_id.eq(&session_id))
        .select(terminal_topology_snapshots::payload_schema_id)
        .first::<Option<String>>(&mut connection)
        .expect("topology schema id should load");
    assert_eq!(topology_schema_id.as_deref(), Some(PAYLOAD_SCHEMA_TOPOLOGY_SNAPSHOT_V1));

    let integrity = store.run_integrity_check().expect("integrity check should run");
    assert_eq!(integrity.result, "passed");
}

#[test]
fn integrity_check_detects_checksum_mismatch() {
    let store = test_store("integrity-mismatch");
    let (session_id, pane_id, writer) = session_and_pane(&store);
    store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id,
            writer.id,
            b"tamper target\r\n".to_vec(),
        ))
        .expect("segment should persist");
    let mut connection = store.connection().expect("connection should open");
    diesel::update(terminal_stream_segments::table)
        .set(terminal_stream_segments::checksum.eq("not-the-real-checksum"))
        .execute(&mut connection)
        .expect("test should corrupt checksum");

    let integrity = store.run_integrity_check().expect("integrity check should run");

    assert_eq!(integrity.result, "failed");
    let error = integrity.error.as_deref().unwrap_or_default();
    assert!(error.contains("history_validation_failures=1"));
    assert!(error.contains("checksum_failures=1"));
    let health = store.list_open_data_health_records(None).expect("health records should list");
    assert_eq!(health.len(), 1);
    assert_eq!(health[0].detection_kind, "checksum_mismatch");
    assert_eq!(health[0].severity, "critical");
    assert_eq!(health[0].action_state, "quarantined");
    assert!(health[0].affected_ref.as_deref().unwrap_or_default().contains("stream_segment"));

    let duplicate_integrity = store.run_integrity_check().expect("second check should run");
    let duplicate_health =
        store.list_open_data_health_records(None).expect("health records should list");
    assert_eq!(duplicate_integrity.result, "failed");
    assert_eq!(duplicate_health.len(), 1);
}

#[test]
fn hydrate_pane_history_quarantines_corrupt_segments_as_visible_gaps() {
    let store = test_store("hydrate-corrupt-segment");
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
            writer.id,
            b"still visible\r\n".to_vec(),
        ))
        .expect("healthy segment should persist");
    let mut connection = store.connection().expect("connection should open");
    diesel::update(
        terminal_stream_segments::table
            .filter(terminal_stream_segments::id.eq(&corrupt.segment_id)),
    )
    .set(terminal_stream_segments::checksum.eq("not-the-real-checksum"))
    .execute(&mut connection)
    .expect("test should corrupt checksum");

    let hydrated = store
        .hydrate_pane_history(&session_id, &pane_id, Some(1), Some(10), Some(1024))
        .expect("hydration should degrade instead of returning corrupt bytes");

    assert_eq!(hydrated.segments.len(), 1);
    assert_eq!(hydrated.segments[0].id, healthy.segment_id);
    assert_eq!(hydrated.segments[0].payload, b"still visible\r\n");
    assert!(hydrated.gaps.iter().any(|gap| {
        gap.gap_kind == "corrupted_segment"
            && gap.event_seq_low == Some(corrupt.event_seq_low)
            && gap.event_seq_high == Some(corrupt.event_seq_high)
    }));

    let health =
        store.list_open_data_health_records(Some(&session_id)).expect("health records should list");
    assert_eq!(health.len(), 1);
    assert_eq!(health[0].detection_kind, "checksum_mismatch");
    assert_eq!(health[0].action_state, "quarantined");
    assert_eq!(hydrated.restore_plan.guarantee_level, RestoreGuaranteeLevel::DegradedHistory);
}

#[test]
fn restore_plan_downgrades_on_open_critical_health_records() {
    let store = test_store("restore-health-downgrade");
    let (session_id, pane_id, writer) = session_and_pane(&store);
    let output = store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id.clone(),
            writer.id.clone(),
            b"health downgrade target\r\n".to_vec(),
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
            screen: serde_json::json!({"lines":["health downgrade target"]}),
            parser_version: None,
            projection_version: None,
            metadata: None,
        })
        .expect("screen snapshot should persist");

    let before_health = store.restore_plan(&session_id).expect("plan should load");
    assert_eq!(before_health.guarantee_level, RestoreGuaranteeLevel::BasicHistory);

    let mut connection = store.connection().expect("connection should open");
    diesel::update(
        terminal_stream_segments::table.filter(terminal_stream_segments::id.eq(output.segment_id)),
    )
    .set(terminal_stream_segments::checksum.eq("not-the-real-checksum"))
    .execute(&mut connection)
    .expect("test should corrupt checksum");
    store.run_integrity_check().expect("integrity check should persist health");

    let plan = store.restore_plan(&session_id).expect("plan should reload");

    assert_eq!(plan.guarantee_level, RestoreGuaranteeLevel::DegradedHistory);
    assert!(plan.evidence.iter().any(|evidence| {
        evidence.kind == "critical_data_health_record_count" && evidence.value == "1"
    }));
}
