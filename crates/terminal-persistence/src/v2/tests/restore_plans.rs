use super::super::*;
use super::support::*;

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

#[test]
fn restore_plan_promotes_raw_stream_after_drill_and_fresh_capability() {
    let store = test_store("restore-plan-raw-evidence");
    let (session_id, pane_id, writer) = session_and_pane(&store);
    let segment = store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id.clone(),
            writer.id.clone(),
            b"raw durable history\r\n".to_vec(),
        ))
        .expect("raw segment should persist");
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
            screen: serde_json::json!({"lines":["raw durable history"]}),
            parser_version: None,
            projection_version: None,
            metadata: None,
        })
        .expect("screen snapshot should persist");
    store
        .write_topology_snapshot(TopologySnapshotInput {
            id: None,
            session_id: session_id.clone(),
            writer_generation: writer.id.clone(),
            pane_high_water: serde_json::json!({ pane_id.clone(): segment.event_seq_high }),
            topology: serde_json::json!({"tabs":[{"active_pane_id": pane_id}]}),
            source: None,
            metadata: None,
        })
        .expect("topology snapshot should persist");
    let capability_id = store
        .record_backend_capability_report(BackendCapabilityReportInput {
            id: None,
            session_id: Some(session_id.clone()),
            backend_kind: "native".to_string(),
            backend_version: Some("test".to_string()),
            backend_binary_path_hash: Some("test-path-hash".to_string()),
            route_kind: "local_daemon".to_string(),
            probe_status: "passed".to_string(),
            capture_strategy: "raw_stream".to_string(),
            capture_semantics: "raw_vt_stream".to_string(),
            can_preserve_process_when_live: false,
            can_capture_scrollback: true,
            command_boundary_confidence: "high".to_string(),
            evidence: Some(serde_json::json!({"probe": "test"})),
            expires_at_ms: None,
        })
        .expect("capability report should persist");

    let before_drill = store.restore_plan(&session_id).expect("plan should load");
    assert_eq!(before_drill.guarantee_level, RestoreGuaranteeLevel::BasicHistory);

    let drill = store.run_restore_drill(&session_id).expect("restore drill should pass");
    assert_eq!(drill.result, "passed");

    let plan = store.restore_plan(&session_id).expect("plan should reload");

    assert_eq!(plan.guarantee_level, RestoreGuaranteeLevel::RawStreamReplay);
    assert_eq!(plan.latest_restore_drill_status.as_deref(), Some("passed"));
    assert!(plan.evidence.iter().any(|evidence| {
        evidence.kind == "backend_capability_report" && evidence.value == capability_id
    }));
    assert!(
        plan.evidence
            .iter()
            .any(|evidence| evidence.kind == "restore_drill" && evidence.value == drill.id)
    );
    assert!(
        plan.evidence.iter().any(|evidence| {
            evidence.kind == "raw_stream_segment_count" && evidence.value == "1"
        })
    );
}

#[test]
fn force_disabled_authoritative_reads_downgrades_restore_plan() {
    let store = test_store("restore-plan-force-disabled");
    let (session_id, pane_id, writer) = session_and_pane(&store);
    let segment = store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id.clone(),
            writer.id.clone(),
            b"raw history\r\n".to_vec(),
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
            screen: serde_json::json!({"lines":["raw history"]}),
            parser_version: None,
            projection_version: None,
            metadata: None,
        })
        .expect("screen snapshot should persist");
    store
        .set_feature_gate_state(
            FeatureGateName::TerminalPersistenceV2AuthoritativeReads,
            FeatureGateState::ForceDisabled,
            Some("test rollback"),
        )
        .expect("force disabled gate should persist");

    let plan = store.restore_plan(&session_id).expect("plan should load");

    assert_eq!(plan.guarantee_level, RestoreGuaranteeLevel::DegradedHistory);
    assert!(plan.evidence.iter().any(|evidence| {
        evidence.kind == "authoritative_reads_gate_state"
            && evidence.value == FeatureGateState::ForceDisabled.as_str()
    }));
}

#[test]
fn stale_backend_capability_report_downgrades_restore_plan() {
    let store = test_store("capability-downgrade");
    let (session_id, pane_id, writer) = session_and_pane(&store);
    store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id.clone(),
            writer.id.clone(),
            b"mux rendered history\r\n".to_vec(),
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
            base_event_seq: 1,
            high_water_event_seq: 1,
            high_water_byte_seq: Some(22),
            screen: serde_json::json!({"lines":["mux rendered history"]}),
            parser_version: None,
            projection_version: None,
            metadata: None,
        })
        .expect("screen snapshot should persist");
    store
        .record_backend_capability_report(BackendCapabilityReportInput {
            id: None,
            session_id: Some(session_id.clone()),
            backend_kind: "zellij".to_string(),
            backend_version: Some("test".to_string()),
            backend_binary_path_hash: Some("test-path-hash".to_string()),
            route_kind: "imported_foreign".to_string(),
            probe_status: "passed".to_string(),
            capture_strategy: "rendered_stream".to_string(),
            capture_semantics: "rendered_plaintext_snapshot".to_string(),
            can_preserve_process_when_live: true,
            can_capture_scrollback: true,
            command_boundary_confidence: "low".to_string(),
            evidence: Some(serde_json::json!({"probe": "test"})),
            expires_at_ms: Some(1),
        })
        .expect("capability report should persist");

    let plan = store.restore_plan(&session_id).expect("restore plan should load");

    assert_eq!(plan.guarantee_level, RestoreGuaranteeLevel::DegradedHistory);
    assert!(plan.evidence.iter().any(|evidence| {
        evidence.kind == "backend_capture_semantics"
            && evidence.value == "rendered_plaintext_snapshot"
    }));
    assert!(plan.evidence.iter().any(|evidence| {
        evidence.kind == "backend_capability_stale" && evidence.value == "true"
    }));
}

#[test]
fn backend_capability_drift_invalidation_marks_reports_stale() {
    let store = test_store("backend-capability-drift");
    let (session_id, pane_id, writer) = session_and_pane(&store);
    let mut segment_input = StreamSegmentInput::terminal_output(
        session_id.clone(),
        pane_id.clone(),
        writer.id.clone(),
        b"zellij rendered history\r\n".to_vec(),
    );
    segment_input.capture_semantics = Some("rendered_plaintext_snapshot".to_string());
    let segment = store.append_stream_segment(segment_input).expect("segment should persist");
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
            base_event_seq: segment.event_seq_low,
            high_water_event_seq: segment.event_seq_high,
            high_water_byte_seq: Some(segment.byte_high),
            screen: serde_json::json!({"lines":["zellij rendered history"]}),
            parser_version: None,
            projection_version: None,
            metadata: None,
        })
        .expect("screen snapshot should persist");
    store
        .record_backend_capability_report(BackendCapabilityReportInput {
            id: None,
            session_id: Some(session_id.clone()),
            backend_kind: "zellij".to_string(),
            backend_version: Some("0.44.1".to_string()),
            backend_binary_path_hash: Some("old-path-hash".to_string()),
            route_kind: "imported_foreign".to_string(),
            probe_status: "passed".to_string(),
            capture_strategy: "rendered_snapshot".to_string(),
            capture_semantics: "rendered_plaintext_snapshot".to_string(),
            can_preserve_process_when_live: true,
            can_capture_scrollback: true,
            command_boundary_confidence: "low".to_string(),
            evidence: Some(serde_json::json!({"probe": "zellij"})),
            expires_at_ms: None,
        })
        .expect("capability report should persist");

    let updated = store
        .mark_backend_capability_reports_stale(BackendCapabilityStaleInput {
            session_id: Some(session_id.clone()),
            backend_kind: Some("zellij".to_string()),
            route_kind: Some("imported_foreign".to_string()),
            stale_reason: "backend_version_changed".to_string(),
        })
        .expect("capability reports should mark stale");
    let second_update = store
        .mark_backend_capability_reports_stale(BackendCapabilityStaleInput {
            session_id: Some(session_id.clone()),
            backend_kind: Some("zellij".to_string()),
            route_kind: Some("imported_foreign".to_string()),
            stale_reason: "backend_version_changed".to_string(),
        })
        .expect("already stale reports should not update again");
    let plan = store.restore_plan(&session_id).expect("restore plan should load");
    let bad_reason = store
        .mark_backend_capability_reports_stale(BackendCapabilityStaleInput {
            session_id: Some(session_id),
            backend_kind: Some("zellij".to_string()),
            route_kind: Some("imported_foreign".to_string()),
            stale_reason: "maybe".to_string(),
        })
        .expect_err("unknown stale reason should fail");

    assert_eq!(updated, 1);
    assert_eq!(second_update, 0);
    assert_eq!(plan.guarantee_level, RestoreGuaranteeLevel::DegradedHistory);
    assert!(plan.evidence.iter().any(|evidence| {
        evidence.kind == "backend_capability_stale_reason"
            && evidence.value == "backend_version_changed"
    }));
    assert!(
        matches!(bad_reason, TerminalPersistenceV2Error::InvalidData(message) if message.contains("stale reason"))
    );
}

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
