use super::super::super::*;
use super::super::support::*;

#[test]
fn creates_vacuum_backup_that_reopens_with_history() {
    let store = test_store("vacuum-backup");
    let (session_id, pane_id, writer) = session_and_pane(&store);
    store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id.clone(),
            writer.id,
            b"backup history\r\n".to_vec(),
        ))
        .expect("segment should persist");
    let target_path = std::env::temp_dir()
        .join(format!("terminal-persistence-v2-backup-{}.sqlite3", Uuid::new_v4()));

    let backup = store.vacuum_into_backup(&target_path).expect("backup should succeed");
    let backup_store =
        TerminalPersistenceV2::open_with_config(&target_path, TerminalPersistenceV2Config::test())
            .expect("backup should reopen");
    let segments = backup_store
        .list_stream_segments(&session_id, &pane_id, 1, 10)
        .expect("backup should contain history");
    let payload = segments.into_iter().flat_map(|segment| segment.payload).collect::<Vec<_>>();

    assert_eq!(backup.state, "succeeded");
    assert_eq!(backup.quick_check_result.as_deref(), Some("ok"));
    assert_eq!(payload, b"backup history\r\n");

    let _ = std::fs::remove_file(&target_path);
    let _ = std::fs::remove_file(target_path.with_extension("sqlite3-wal"));
    let _ = std::fs::remove_file(target_path.with_extension("sqlite3-shm"));
}

#[test]
fn vacuum_backup_rejects_live_database_and_sidecar_targets() {
    let store = test_store("vacuum-backup-target-guard");
    let live_db = store.path().to_path_buf();
    let wal_sidecar = sqlite_sidecar_path(store.path(), "-wal");
    let shm_sidecar = sqlite_sidecar_path(store.path(), "-shm");

    for target in [live_db, wal_sidecar, shm_sidecar] {
        let error = store
            .vacuum_into_backup(&target)
            .expect_err("live database and sidecar targets should be rejected");
        assert!(matches!(
            error,
            TerminalPersistenceV2Error::InvalidData(message)
                if message.contains("live database or SQLite sidecar")
        ));
    }
}

#[test]
fn maintenance_run_records_checkpoint_and_optimize_audit() {
    let store = test_store("maintenance-run");
    let (session_id, pane_id, writer) = session_and_pane(&store);
    store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id,
            pane_id,
            writer.id,
            b"maintenance history\r\n".to_vec(),
        ))
        .expect("segment should persist before maintenance");

    let run =
        store.run_maintenance(MaintenanceRunInput::default()).expect("maintenance should run");
    let summary = run.summary_json.as_ref().expect("maintenance summary should exist");

    assert_eq!(run.state, "succeeded");
    assert_eq!(run.run_kind, "scheduled_maintenance");
    assert!(run.finished_at_ms.unwrap_or_default() >= run.started_at_ms);
    assert_eq!(summary["wal_checkpoint"]["mode"], "PASSIVE");
    assert!(summary["wal_checkpoint"]["log_frames"].as_i64().is_some());
    assert_eq!(summary["optimize"]["ran"], true);
    assert_eq!(summary["outbox"]["pending_count"], 1);
    assert_eq!(summary["outbox"]["due_pending_count"], 1);
    assert_eq!(summary["compression"]["feature_gate_state"], "disabled");
    assert_eq!(summary["compression"]["raw_segment_count"], 1);
    assert_eq!(summary["compression"]["segments_rewritten"], 0);
    assert_eq!(summary["compression"]["action_taken"], "skipped_feature_disabled");
    assert_eq!(summary["retention"]["policy_id"], DEFAULT_RETENTION_POLICY_ID);
    assert_eq!(summary["retention"]["scan_mode"], "warn_only");
    assert_eq!(summary["retention"]["maintenance_deletes_raw_history"], false);
    assert_eq!(summary["retention"]["sessions_scanned"], 1);
    assert_eq!(summary["retention"]["action_taken"], "warn_only_no_delete");
    assert_eq!(summary["storage"]["no_silent_delete"], true);
}

#[test]
fn compression_diagnostics_never_rewrites_segments_without_restore_guard() {
    let store = test_store("compression-placeholder");
    store
        .set_feature_gate_state(
            FeatureGateName::SegmentCompressionZstd,
            FeatureGateState::Enabled,
            Some("test"),
        )
        .expect("compression gate should enable");
    let (session_id, pane_id, writer) = session_and_pane(&store);
    let receipt = store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id.clone(),
            writer.id,
            b"raw segment remains raw\r\n".to_vec(),
        ))
        .expect("segment should persist");

    let diagnostics = store.compression_diagnostics().expect("compression diagnostics should load");
    let run = store
        .run_maintenance(MaintenanceRunInput {
            run_wal_checkpoint: false,
            run_optimize: false,
            ..MaintenanceRunInput::default()
        })
        .expect("maintenance should run");
    let summary = run.summary_json.as_ref().expect("maintenance summary should exist");
    let mut connection = store.connection().expect("connection should open");
    let compression = terminal_stream_segments::table
        .filter(terminal_stream_segments::id.eq(&receipt.segment_id))
        .select(terminal_stream_segments::compression)
        .first::<String>(&mut connection)
        .expect("segment compression should load");

    assert_eq!(diagnostics.feature_gate_state, "enabled");
    assert_eq!(diagnostics.raw_segment_count, 1);
    assert_eq!(diagnostics.rewrite_candidate_count, 1);
    assert_eq!(diagnostics.segments_rewritten, 0);
    assert_eq!(diagnostics.action_taken, "skipped_restore_drill_guard");
    assert_eq!(summary["compression"]["feature_gate_state"], "enabled");
    assert_eq!(summary["compression"]["rewrite_candidate_count"], 1);
    assert_eq!(summary["compression"]["segments_rewritten"], 0);
    assert_eq!(summary["compression"]["action_taken"], "skipped_restore_drill_guard");
    assert_eq!(compression, "none");
}

#[test]
fn maintenance_requeues_stale_outbox_claims_and_marks_stale_writers() {
    let store = test_store("maintenance-recovery");
    let message = store
        .enqueue_outbox_message(OutboxMessageInput {
            message_kind: "restore_drill".to_string(),
            payload: serde_json::json!({ "session_id": "session-a" }),
            dedupe_key: None,
            max_attempts: Some(2),
            next_run_at_ms: None,
        })
        .expect("message should enqueue");
    let first_claim = store
        .claim_next_outbox_message("worker-a", 60_000)
        .expect("claim should succeed")
        .expect("message should be claimable");
    let writer =
        store.acquire_writer_generation("process-a", 60_000).expect("writer should acquire");
    let expired_at_ms = store.config.clock.now_ms() - 1;

    {
        let mut connection = store.connection().expect("connection should open");
        diesel::update(
            terminal_outbox_messages::table.filter(terminal_outbox_messages::id.eq(&message.id)),
        )
        .set((
            terminal_outbox_messages::claimed_until_ms.eq(Some(expired_at_ms)),
            terminal_outbox_messages::updated_at_ms.eq(expired_at_ms),
        ))
        .execute(&mut connection)
        .expect("test should expire outbox lease");
        diesel::update(
            terminal_writer_generations::table
                .filter(terminal_writer_generations::id.eq(&writer.id)),
        )
        .set((
            terminal_writer_generations::heartbeat_at_ms.eq(expired_at_ms),
            terminal_writer_generations::lease_expires_at_ms.eq(expired_at_ms),
        ))
        .execute(&mut connection)
        .expect("test should expire writer lease");
    }
    let before_maintenance = store.outbox_diagnostics().expect("outbox diagnostics should load");

    assert_eq!(before_maintenance.claimed_count, 1);
    assert_eq!(before_maintenance.stale_claim_count, 1);

    let run = store
        .run_maintenance(MaintenanceRunInput {
            run_wal_checkpoint: false,
            run_optimize: false,
            ..MaintenanceRunInput::default()
        })
        .expect("maintenance should recover stale leases");
    let summary = run.summary_json.as_ref().expect("maintenance summary should exist");
    let second_claim = store
        .claim_next_outbox_message("worker-b", 60_000)
        .expect("second claim should succeed")
        .expect("stale outbox message should be requeued");
    let replacement_writer = store
        .acquire_writer_generation("process-b", 60_000)
        .expect("new writer should acquire after stale recovery");

    assert_ne!(first_claim.lease_token, second_claim.lease_token);
    assert_eq!(second_claim.id, message.id);
    assert_eq!(second_claim.state, "claimed");
    assert_eq!(second_claim.claimed_by.as_deref(), Some("worker-b"));
    assert_eq!(summary["recovery"]["stale_outbox_claims_requeued"], 1);
    assert_eq!(summary["recovery"]["stale_outbox_claims_quarantined"], 0);
    assert_eq!(summary["recovery"]["stale_writer_generations_marked"], 1);
    assert_eq!(summary["outbox"]["pending_count"], 1);
    assert_eq!(summary["outbox"]["due_pending_count"], 1);
    assert_eq!(summary["outbox"]["stale_claim_count"], 0);

    let mut connection = store.connection().expect("connection should open");
    let stale_writer_state = terminal_writer_generations::table
        .filter(terminal_writer_generations::id.eq(&writer.id))
        .select(terminal_writer_generations::state)
        .first::<String>(&mut connection)
        .expect("stale writer should load");
    let recovery_anchor_count = terminal_clock_anchors::table
        .filter(terminal_clock_anchors::writer_generation.eq(&writer.id))
        .filter(terminal_clock_anchors::source.eq("writer_stale_recovery"))
        .count()
        .get_result::<i64>(&mut connection)
        .expect("recovery anchor should count");

    assert_eq!(stale_writer_state, "stale");
    assert_eq!(recovery_anchor_count, 1);
    store
        .release_writer_generation(&replacement_writer.id)
        .expect("replacement writer should release");
}
