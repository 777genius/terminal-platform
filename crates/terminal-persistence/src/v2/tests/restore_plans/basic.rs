use super::super::super::*;
use super::super::support::*;

#[test]
fn restore_plan_uses_snapshots_and_stream_evidence() {
    let store = test_store("restore-plan");
    let (session_id, pane_id, writer) = session_and_pane(&store);
    store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id.clone(),
            writer.id.clone(),
            b"visible history\r\n".to_vec(),
        ))
        .expect("segment should persist");
    let screen_id = store
        .write_screen_snapshot(ScreenSnapshotInput {
            id: None,
            session_id: session_id.clone(),
            pane_id,
            writer_generation: writer.id.clone(),
            projection_source: None,
            buffer_kind: None,
            rows: 24,
            cols: 80,
            base_event_seq: 1,
            high_water_event_seq: 1,
            high_water_byte_seq: Some(17),
            screen: serde_json::json!({"lines":["visible history"]}),
            parser_version: None,
            projection_version: None,
            metadata: None,
        })
        .expect("screen snapshot should persist");
    let topology_id = store
        .write_topology_snapshot(TopologySnapshotInput {
            id: None,
            session_id: session_id.clone(),
            writer_generation: writer.id,
            pane_high_water: serde_json::json!({}),
            topology: serde_json::json!({"tabs":[]}),
            source: None,
            metadata: None,
        })
        .expect("topology snapshot should persist");

    let plan = store.restore_plan(&session_id).expect("restore plan should load");

    assert_eq!(plan.guarantee_level, RestoreGuaranteeLevel::BasicHistory);
    assert_eq!(plan.latest_screen_snapshot_id, Some(screen_id.clone()));
    assert_eq!(plan.latest_topology_snapshot_id, Some(topology_id.clone()));
    assert!(plan.high_water_commit_seq >= 3);
    assert_eq!(plan.latest_restore_drill_status, None);
    assert!(plan.evidence.iter().any(|evidence| {
        evidence.kind == "authoritative_reads_gate_state" && evidence.value == "disabled"
    }));
    assert!(
        plan.evidence
            .iter()
            .any(|evidence| { evidence.kind == "screen_snapshot" && evidence.value == screen_id })
    );
    assert!(
        plan.evidence.iter().any(|evidence| {
            evidence.kind == "topology_snapshot" && evidence.value == topology_id
        })
    );
    assert!(plan.evidence.iter().any(|evidence| evidence.kind == "journal_event_range"));
}

#[test]
fn restore_plan_and_hydration_respect_topology_high_water_vector() {
    let store = test_store("restore-topology-high-water");
    let (session_id, pane_id, writer) = session_and_pane(&store);
    let first = store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id.clone(),
            writer.id.clone(),
            b"topology-consistent\r\n".to_vec(),
        ))
        .expect("first segment should persist");
    let topology_consistent_screen = store
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
            screen: serde_json::json!({"lines":["topology-consistent"]}),
            parser_version: None,
            projection_version: None,
            metadata: None,
        })
        .expect("topology-consistent screen snapshot should persist");
    let second = store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id.clone(),
            writer.id.clone(),
            b"too-new-for-topology\r\n".to_vec(),
        ))
        .expect("second segment should persist");
    let too_new_screen = store
        .write_screen_snapshot(ScreenSnapshotInput {
            id: None,
            session_id: session_id.clone(),
            pane_id: pane_id.clone(),
            writer_generation: writer.id.clone(),
            projection_source: None,
            buffer_kind: None,
            rows: 24,
            cols: 80,
            base_event_seq: second.event_seq_low,
            high_water_event_seq: second.event_seq_high,
            high_water_byte_seq: Some(second.byte_high),
            screen: serde_json::json!({"lines":["too-new-for-topology"]}),
            parser_version: None,
            projection_version: None,
            metadata: None,
        })
        .expect("newer screen snapshot should persist");
    let topology = store
        .write_topology_snapshot(TopologySnapshotInput {
            id: None,
            session_id: session_id.clone(),
            writer_generation: writer.id,
            pane_high_water: serde_json::json!({ pane_id.clone(): first.event_seq_high }),
            topology: serde_json::json!({"tabs":[{"active_pane_id": pane_id.clone()}]}),
            source: None,
            metadata: None,
        })
        .expect("topology snapshot should persist");

    let plan = store.restore_plan(&session_id).expect("restore plan should load");
    let hydrated = store
        .hydrate_pane_history(&session_id, &pane_id, Some(1), Some(10), Some(1024))
        .expect("pane history should hydrate");

    assert_eq!(
        plan.latest_screen_snapshot_id.as_deref(),
        Some(topology_consistent_screen.as_str())
    );
    assert_eq!(plan.latest_topology_snapshot_id.as_deref(), Some(topology.as_str()));
    assert!(!plan.evidence.iter().any(|evidence| {
        evidence.kind == "screen_snapshot" && evidence.value == too_new_screen
    }));
    assert_eq!(
        hydrated.latest_screen_snapshot.as_ref().map(|snapshot| snapshot.id.as_str()),
        Some(topology_consistent_screen.as_str())
    );
    let health = store
        .list_open_data_health_records(Some(&session_id))
        .expect("projection health records should list");
    assert_eq!(health.len(), 1);
    assert_eq!(health[0].detection_kind, "projection_drift");
    assert_eq!(health[0].action_state, "rebuild_pending");
    assert!(
        health[0]
            .affected_ref
            .as_deref()
            .unwrap_or_default()
            .contains("topology high_water_event_seq")
    );
}

#[test]
fn runtime_topology_snapshot_records_persisted_pane_high_water() {
    let store = test_store("runtime-topology-high-water");
    let (session_id, pane_id, writer) = session_and_pane(&store);
    let segment = store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id.clone(),
            writer.id.clone(),
            b"topology runtime high water\r\n".to_vec(),
        ))
        .expect("segment should persist");
    store.release_writer_generation(&writer.id).expect("writer should release");

    let session_typed = SessionId(Uuid::parse_str(&session_id).expect("session id should be uuid"));
    let pane_typed = PaneId(Uuid::parse_str(&pane_id).expect("pane id should be uuid"));
    let tab_id = TabId::new();
    let topology_id = store
        .record_topology_snapshot_event(TopologySnapshotEventInput {
            session_id: session_id.clone(),
            route: route(),
            title: Some("runtime topology".to_string()),
            launch: None,
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
        })
        .expect("runtime topology snapshot should persist");

    let mut connection = store.connection().expect("connection should open");
    let pane_high_water_json = terminal_topology_snapshots::table
        .filter(terminal_topology_snapshots::id.eq(topology_id))
        .select(terminal_topology_snapshots::pane_high_water_json)
        .first::<String>(&mut connection)
        .expect("topology high-water should load");
    let high_water =
        parse_pane_high_water_json(&pane_high_water_json).expect("high-water should parse");

    assert_eq!(high_water.get(&pane_id), Some(&segment.event_seq_high));
}
