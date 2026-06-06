use super::super::super::*;
use super::super::support::*;

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
        evidence.kind == "backend_can_preserve_process_when_live" && evidence.value == "true"
    }));
    assert!(plan.evidence.iter().any(|evidence| {
        evidence.kind == "backend_can_capture_scrollback" && evidence.value == "true"
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
