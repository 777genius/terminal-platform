use super::super::super::*;
use super::super::support::*;

#[test]
fn hydrate_pane_history_skips_corrupt_latest_screen_snapshot() {
    let store = test_store("restore-screen-snapshot-fallback");
    let (session_id, pane_id, writer) = session_and_pane(&store);
    let first = store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id.clone(),
            writer.id.clone(),
            b"valid snapshot base\r\n".to_vec(),
        ))
        .expect("first segment should persist");
    let valid_snapshot = store
        .write_screen_snapshot(ScreenSnapshotInput {
            id: None,
            session_id: session_id.clone(),
            pane_id: pane_id.clone(),
            writer_generation: writer.id.clone(),
            projection_source: None,
            buffer_kind: None,
            rows: 24,
            cols: 80,
            base_event_seq: first.event_seq_low,
            high_water_event_seq: first.event_seq_high,
            high_water_byte_seq: Some(first.byte_high),
            screen: serde_json::json!({"lines":["valid snapshot base"]}),
            parser_version: None,
            projection_version: None,
            metadata: None,
        })
        .expect("valid screen snapshot should persist");
    let second = store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id.clone(),
            writer.id.clone(),
            b"corrupt snapshot tip\r\n".to_vec(),
        ))
        .expect("second segment should persist");
    let corrupt_snapshot = store
        .write_screen_snapshot(ScreenSnapshotInput {
            id: None,
            session_id: session_id.clone(),
            pane_id: pane_id.clone(),
            writer_generation: writer.id,
            projection_source: None,
            buffer_kind: None,
            rows: 24,
            cols: 80,
            base_event_seq: second.event_seq_low,
            high_water_event_seq: second.event_seq_high,
            high_water_byte_seq: Some(second.byte_high),
            screen: serde_json::json!({"lines":["corrupt snapshot tip"]}),
            parser_version: None,
            projection_version: None,
            metadata: None,
        })
        .expect("corrupt screen snapshot candidate should persist");
    let mut connection = store.connection().expect("connection should open");
    diesel::update(
        terminal_screen_snapshots::table
            .filter(terminal_screen_snapshots::id.eq(&corrupt_snapshot)),
    )
    .set(terminal_screen_snapshots::checksum.eq("not-the-real-checksum"))
    .execute(&mut connection)
    .expect("test should corrupt latest screen snapshot");

    let hydrated = store
        .hydrate_pane_history(&session_id, &pane_id, Some(1), Some(10), Some(1024))
        .expect("hydration should skip corrupt latest snapshot");
    let plan = store.restore_plan(&session_id).expect("restore plan should skip corrupt snapshot");

    assert_eq!(
        hydrated.latest_screen_snapshot.as_ref().map(|snapshot| snapshot.id.as_str()),
        Some(valid_snapshot.as_str())
    );
    assert_eq!(hydrated.segments.len(), 2);
    assert_eq!(plan.latest_screen_snapshot_id.as_deref(), Some(valid_snapshot.as_str()));
    assert!(!plan.evidence.iter().any(|evidence| {
        evidence.kind == "screen_snapshot" && evidence.value == corrupt_snapshot
    }));

    let health = store
        .list_open_data_health_records(Some(&session_id))
        .expect("snapshot health records should list");
    assert_eq!(health.len(), 1);
    assert_eq!(health[0].detection_kind, "checksum_mismatch");
    assert_eq!(health[0].severity, "error");
    assert_eq!(health[0].action_state, "rebuild_pending");
    assert!(health[0].affected_ref.as_deref().unwrap_or_default().contains("screen_snapshot"));
}

#[test]
fn restore_plan_skips_corrupt_latest_topology_snapshot() {
    let store = test_store("restore-topology-snapshot-fallback");
    let (session_id, pane_id, writer) = session_and_pane(&store);
    let segment = store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id.clone(),
            writer.id.clone(),
            b"topology fallback\r\n".to_vec(),
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
            base_event_seq: segment.event_seq_low,
            high_water_event_seq: segment.event_seq_high,
            high_water_byte_seq: Some(segment.byte_high),
            screen: serde_json::json!({"lines":["topology fallback"]}),
            parser_version: None,
            projection_version: None,
            metadata: None,
        })
        .expect("screen snapshot should persist");
    let valid_topology = store
        .write_topology_snapshot(TopologySnapshotInput {
            id: None,
            session_id: session_id.clone(),
            writer_generation: writer.id.clone(),
            pane_high_water: serde_json::json!({ pane_id.clone(): segment.event_seq_high }),
            topology: serde_json::json!({"tabs":[{"active_pane_id": pane_id.clone()}]}),
            source: None,
            metadata: None,
        })
        .expect("valid topology snapshot should persist");
    let corrupt_topology = store
        .write_topology_snapshot(TopologySnapshotInput {
            id: None,
            session_id: session_id.clone(),
            writer_generation: writer.id,
            pane_high_water: serde_json::json!({ pane_id.clone(): segment.event_seq_high }),
            topology: serde_json::json!({"tabs":[{"active_pane_id": pane_id}],"tip":true}),
            source: None,
            metadata: None,
        })
        .expect("corrupt topology snapshot candidate should persist");
    let mut connection = store.connection().expect("connection should open");
    diesel::update(
        terminal_topology_snapshots::table
            .filter(terminal_topology_snapshots::id.eq(&corrupt_topology)),
    )
    .set(terminal_topology_snapshots::checksum.eq("not-the-real-checksum"))
    .execute(&mut connection)
    .expect("test should corrupt latest topology snapshot");

    let plan = store.restore_plan(&session_id).expect("restore plan should load");

    assert_eq!(plan.guarantee_level, RestoreGuaranteeLevel::BasicHistory);
    assert_eq!(plan.latest_topology_snapshot_id.as_deref(), Some(valid_topology.as_str()));
    assert!(!plan.evidence.iter().any(|evidence| {
        evidence.kind == "topology_snapshot" && evidence.value == corrupt_topology
    }));
    let health = store
        .list_open_data_health_records(Some(&session_id))
        .expect("topology health records should list");
    assert_eq!(health.len(), 1);
    assert_eq!(health[0].detection_kind, "checksum_mismatch");
    assert_eq!(health[0].action_state, "rebuild_pending");
    assert!(health[0].affected_ref.as_deref().unwrap_or_default().contains("topology_snapshot"));
}
