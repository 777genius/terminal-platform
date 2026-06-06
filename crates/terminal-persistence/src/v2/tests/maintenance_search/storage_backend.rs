use super::super::super::*;
use super::super::support::*;

#[test]
fn storage_probe_records_warning_when_file_budget_is_exceeded() {
    let mut config = TerminalPersistenceV2Config::test();
    config.storage_pressure.db_warning_bytes = 1;
    config.storage_pressure.wal_warning_bytes = i64::MAX;
    let path = std::env::temp_dir()
        .join(format!("terminal-persistence-v2-storage-pressure-{}.sqlite3", Uuid::new_v4()));
    let store = TerminalPersistenceV2::open_with_config(path, config).expect("store should open");

    let pressure = store.probe_storage_health().expect("storage probe should persist");

    assert_eq!(pressure.state, "warning");
    assert_eq!(pressure.action_taken, "warn_only");
    assert_eq!(pressure.reason.as_deref(), Some("db_file_size_over_budget"));
    assert_eq!(
        pressure.metadata_json.as_ref().and_then(|metadata| metadata["db_over_budget"].as_bool()),
        Some(true)
    );
    assert_eq!(
        pressure.metadata_json.as_ref().and_then(|metadata| metadata["no_silent_delete"].as_bool()),
        Some(true)
    );
}

#[test]
fn storage_pressure_rejects_unknown_domain_values() {
    let store = test_store("storage-pressure-domain");

    let error = store
        .record_storage_pressure_event(StoragePressureEventInput {
            id: None,
            state: Some("maybe_bad".to_string()),
            db_file_bytes: None,
            wal_file_bytes: None,
            disk_free_bytes: None,
            temp_free_bytes: None,
            quota_bytes: None,
            action_taken: Some("none".to_string()),
            reason: Some("test".to_string()),
            metadata: None,
        })
        .expect_err("unknown storage pressure state should fail");

    assert!(
        matches!(error, TerminalPersistenceV2Error::InvalidData(message) if message.contains("unknown storage pressure state"))
    );
}

#[test]
fn storage_pressure_db_constraints_reject_unknown_domain_values() {
    let store = test_store("storage-pressure-db-domain");
    let mut connection = store.connection().expect("connection should open");

    let error = diesel::sql_query(
        "INSERT INTO terminal_storage_pressure_events \
             (id, state, action_taken, created_at_ms) \
             VALUES ('invalid-storage-pressure-domain', 'maybe_bad', 'none', 1)",
    )
    .execute(&mut connection)
    .expect_err("sqlite CHECK constraint should reject unknown storage pressure state");

    assert!(matches!(error, DieselError::DatabaseError(_, _)));
}

#[test]
fn backend_capability_db_constraints_reject_unknown_capture_semantics() {
    let store = test_store("backend-capability-db-domain");
    let mut connection = store.connection().expect("connection should open");

    let error = diesel::sql_query(
        "INSERT INTO terminal_backend_capability_reports \
             (id, backend_kind, route_kind, probe_status, capture_strategy, capture_semantics, \
              can_preserve_process_when_live, can_capture_scrollback, command_boundary_confidence, \
              created_at_ms, expires_at_ms) \
             VALUES ('invalid-backend-capability-domain', 'native', 'local_daemon', 'passed', \
                     'raw_stream', 'probably_plain_text', 0, 0, 'unknown', 1, 2)",
    )
    .execute(&mut connection)
    .expect_err("sqlite CHECK constraint should reject unknown capture semantics");

    assert!(matches!(error, DieselError::DatabaseError(_, _)));
}

#[test]
fn backend_capability_db_constraints_reject_unknown_strategy_and_confidence() {
    let store = test_store("backend-capability-db-domain-more");
    let mut connection = store.connection().expect("connection should open");

    let strategy_error = diesel::sql_query(
        "INSERT INTO terminal_backend_capability_reports \
             (id, backend_kind, route_kind, probe_status, capture_strategy, capture_semantics, \
              can_preserve_process_when_live, can_capture_scrollback, command_boundary_confidence, \
              created_at_ms, expires_at_ms) \
             VALUES ('invalid-backend-strategy-domain', 'native', 'local_daemon', 'passed', \
                     'rawish_stream', 'raw_vt_stream', 0, 1, 'high', 1, 2)",
    )
    .execute(&mut connection)
    .expect_err("sqlite CHECK constraint should reject unknown capture strategy");

    let confidence_error = diesel::sql_query(
        "INSERT INTO terminal_backend_capability_reports \
             (id, backend_kind, route_kind, probe_status, capture_strategy, capture_semantics, \
              can_preserve_process_when_live, can_capture_scrollback, command_boundary_confidence, \
              created_at_ms, expires_at_ms) \
             VALUES ('invalid-backend-confidence-domain', 'native', 'local_daemon', 'passed', \
                     'raw_stream', 'raw_vt_stream', 0, 1, 'maybe', 1, 2)",
    )
    .execute(&mut connection)
    .expect_err("sqlite CHECK constraint should reject unknown command confidence");

    assert!(matches!(strategy_error, DieselError::DatabaseError(_, _)));
    assert!(matches!(confidence_error, DieselError::DatabaseError(_, _)));
}
