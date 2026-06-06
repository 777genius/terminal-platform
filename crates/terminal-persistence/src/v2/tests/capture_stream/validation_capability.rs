use super::super::super::*;
use super::super::support::*;

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
