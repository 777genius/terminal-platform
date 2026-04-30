use super::super::*;
use super::support::*;

#[test]
fn records_history_gaps_as_readable_restore_evidence() {
    let store = test_store("history-gap");
    let (session_id, pane_id, writer) = session_and_pane(&store);
    store.release_writer_generation(&writer.id).expect("writer should release");

    store
        .record_history_gap_event(HistoryGapEventInput {
            session_id: session_id.clone(),
            route: route(),
            title: Some("shell".to_string()),
            launch: None,
            pane_id: pane_id.clone(),
            tab_id: None,
            rows: Some(24),
            cols: Some(80),
            skipped_events: 3,
            estimated_dropped_bytes: Some(128),
            reason: "test_receiver_lag".to_string(),
            occurred_at_ms: Some(42),
        })
        .expect("history gap should persist");

    let hydrated = store
        .hydrate_pane_history(&session_id, &pane_id, Some(1), Some(10), Some(1024))
        .expect("pane history should hydrate");

    assert_eq!(hydrated.gaps.len(), 1);
    assert_eq!(hydrated.gaps[0].event_seq_low, Some(1));
    assert_eq!(hydrated.gaps[0].event_seq_high, Some(3));
    assert_eq!(hydrated.gaps[0].estimated_dropped_events, Some(3));
    assert_eq!(hydrated.gaps[0].reason, "test_receiver_lag");
    assert_eq!(hydrated.replay_strategy, PaneHistoryReplayStrategy::Degraded);
    assert_eq!(hydrated.restore_plan.guarantee_level, RestoreGuaranteeLevel::DegradedHistory);
}

#[test]
fn persists_command_blocks_and_command_history() {
    let store = test_store("command-history");
    let (session_id, pane_id, writer) = session_and_pane(&store);
    let output = store
        .append_stream_segment(StreamSegmentInput::terminal_output(
            session_id.clone(),
            pane_id.clone(),
            writer.id,
            b"hello\r\n".to_vec(),
        ))
        .expect("segment should persist");
    let block_id = store
        .write_command_block(CommandBlockInput {
            id: None,
            session_id: session_id.clone(),
            pane_id: pane_id.clone(),
            commit_id: Some(output.commit_id),
            command_text: Some("echo hello".to_string()),
            display_text: Some("echo hello".to_string()),
            redacted_text: None,
            command_text_source: None,
            trust_level: None,
            state: Some("finished".to_string()),
            cwd: Some("C:\\Users\\User".to_string()),
            cwd_source: Some("shell_integration".to_string()),
            exit_code: Some(0),
            started_event_seq: Some(1),
            submitted_event_seq: Some(1),
            finished_event_seq: Some(1),
            output_event_seq_low: Some(1),
            output_event_seq_high: Some(1),
            output_byte_low: Some(output.byte_low),
            output_byte_high: Some(output.byte_high),
            sensitivity_class: None,
            created_at_ms: None,
            metadata: None,
        })
        .expect("command block should persist");
    let history_id = store
        .upsert_command_history_entry(CommandHistoryEntryInput {
            id: None,
            session_id: Some(session_id.clone()),
            pane_id: Some(pane_id.clone()),
            command_block_id: Some(block_id),
            scope_kind: "session".to_string(),
            command_text: Some("echo hello".to_string()),
            display_text: "echo hello".to_string(),
            redacted_text: None,
            command_hash: Some(blake3_hash_text("echo hello")),
            cwd: Some("C:\\Users\\User".to_string()),
            shell_kind: Some("cmd".to_string()),
            trust_level: None,
            source: None,
            sensitivity_class: None,
            redaction_state: None,
            rerun_policy: None,
            first_used_at_ms: None,
            last_used_at_ms: None,
            use_count: None,
            metadata: None,
        })
        .expect("history should persist");

    let listed = store.list_command_history(Some(&session_id), 10).expect("history should list");

    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, history_id);
    assert_eq!(listed[0].display_text, "echo hello");
    assert_eq!(listed[0].use_count, 1);

    let mut connection = store.connection().expect("connection should open");
    let row = terminal_command_history_entries::table
        .filter(terminal_command_history_entries::id.eq(&history_id))
        .select(CommandHistoryEntryRow::as_select())
        .first::<CommandHistoryEntryRow>(&mut connection)
        .expect("history row should load");
    let notes = terminal_db_identity::table
        .filter(terminal_db_identity::id.eq(1))
        .select(terminal_db_identity::notes)
        .first::<Option<String>>(&mut connection)
        .expect("identity notes should load");
    let notes_value = parse_identity_notes(notes.as_deref());

    assert_eq!(row.command_hash_algorithm, COMMAND_HASH_ALGORITHM);
    assert_eq!(row.command_hash_scope, COMMAND_HASH_SCOPE);
    assert_ne!(row.command_hash, blake3_hash_text("echo hello"));
    assert!(command_hash_key_seed_from_notes(&notes_value).is_some());

    let fallback_limit = store
        .list_command_history(Some(&session_id), -1)
        .expect("invalid history limit should fall back");
    assert_eq!(fallback_limit.len(), 1);
}

#[test]
fn command_history_hashes_are_local_keyed_and_stable_per_store() {
    let store = test_store("command-history-keyed");
    let (session_id, pane_id, _) = session_and_pane(&store);
    let input = || CommandHistoryEntryInput {
        id: None,
        session_id: Some(session_id.clone()),
        pane_id: Some(pane_id.clone()),
        command_block_id: None,
        scope_kind: "session".to_string(),
        command_text: Some("git status".to_string()),
        display_text: "git status".to_string(),
        redacted_text: None,
        command_hash: Some("caller-supplied-hash-must-not-win".to_string()),
        cwd: None,
        shell_kind: Some("cmd".to_string()),
        trust_level: None,
        source: None,
        sensitivity_class: None,
        redaction_state: None,
        rerun_policy: None,
        first_used_at_ms: None,
        last_used_at_ms: None,
        use_count: None,
        metadata: None,
    };

    let first_id = store.upsert_command_history_entry(input()).expect("first history upsert");
    let second_id =
        store.upsert_command_history_entry(input()).expect("second history upsert should dedupe");
    let mut connection = store.connection().expect("connection should open");
    let row = terminal_command_history_entries::table
        .filter(terminal_command_history_entries::id.eq(&first_id))
        .select(CommandHistoryEntryRow::as_select())
        .first::<CommandHistoryEntryRow>(&mut connection)
        .expect("history row should load");

    assert_eq!(first_id, second_id);
    assert_eq!(row.use_count, 2);
    assert_eq!(row.command_hash_algorithm, COMMAND_HASH_ALGORITHM);
    assert_eq!(row.command_hash_scope, COMMAND_HASH_SCOPE);
    assert_ne!(row.command_hash, blake3_hash_text("git status"));
    assert_ne!(row.command_hash, "caller-supplied-hash-must-not-win");

    let other_store = test_store("command-history-keyed-other");
    let (other_session_id, other_pane_id, _) = session_and_pane(&other_store);
    let other_id = other_store
        .upsert_command_history_entry(CommandHistoryEntryInput {
            id: None,
            session_id: Some(other_session_id),
            pane_id: Some(other_pane_id),
            command_block_id: None,
            scope_kind: "session".to_string(),
            command_text: Some("git status".to_string()),
            display_text: "git status".to_string(),
            redacted_text: None,
            command_hash: None,
            cwd: None,
            shell_kind: Some("cmd".to_string()),
            trust_level: None,
            source: None,
            sensitivity_class: None,
            redaction_state: None,
            rerun_policy: None,
            first_used_at_ms: None,
            last_used_at_ms: None,
            use_count: None,
            metadata: None,
        })
        .expect("other store history should persist");
    let mut other_connection = other_store.connection().expect("other connection should open");
    let other_row = terminal_command_history_entries::table
        .filter(terminal_command_history_entries::id.eq(&other_id))
        .select(CommandHistoryEntryRow::as_select())
        .first::<CommandHistoryEntryRow>(&mut other_connection)
        .expect("other history row should load");

    assert_ne!(row.command_hash, other_row.command_hash);
}

#[test]
fn command_output_byte_range_is_half_open() {
    let store = test_store("command-output-byte-range");
    let (session_id, pane_id, _) = session_and_pane(&store);

    let error = store
        .write_command_block(CommandBlockInput {
            id: None,
            session_id,
            pane_id,
            commit_id: None,
            command_text: Some("echo bad range".to_string()),
            display_text: Some("echo bad range".to_string()),
            redacted_text: None,
            command_text_source: None,
            trust_level: None,
            state: Some("finished".to_string()),
            cwd: None,
            cwd_source: None,
            exit_code: Some(0),
            started_event_seq: None,
            submitted_event_seq: None,
            finished_event_seq: None,
            output_event_seq_low: None,
            output_event_seq_high: None,
            output_byte_low: Some(42),
            output_byte_high: Some(42),
            sensitivity_class: None,
            created_at_ms: None,
            metadata: None,
        })
        .expect_err("equal byte range should be rejected before sqlite insert");

    assert!(
        matches!(error, TerminalPersistenceV2Error::InvalidData(message) if message.contains("command output byte range must be empty or half-open"))
    );
}

#[test]
fn records_ui_input_as_verified_command_history() {
    let store = test_store("ui-input");
    let session_id = Uuid::new_v4().to_string();
    let pane_id = Uuid::new_v4().to_string();

    store
        .record_ui_input_event(UiInputEventInput {
            session_id: session_id.clone(),
            route: route(),
            title: Some("shell".to_string()),
            launch: None,
            pane_id: pane_id.clone(),
            data: "git status\r".to_string(),
            is_paste: false,
            source_event_id: None,
            rows: None,
            cols: None,
            shell_kind: Some("cmd".to_string()),
        })
        .expect("ui input should persist");

    let history =
        store.list_command_history(Some(&session_id), 10).expect("command history should load");
    let segments = store
        .list_stream_segments(&session_id, &pane_id, 1, 10)
        .expect("rendered/raw segments query should be valid");

    assert_eq!(history.len(), 1);
    assert_eq!(history[0].display_text, "git status");
    assert!(segments.is_empty());
}

#[test]
fn windows_shell_metadata_profiles_cmd_and_powershell_inputs() {
    let store = test_store("windows-shell-profiles");
    let cmd_session_id = Uuid::new_v4().to_string();
    let cmd_pane_id = Uuid::new_v4().to_string();
    let powershell_session_id = Uuid::new_v4().to_string();
    let powershell_pane_id = Uuid::new_v4().to_string();

    store
        .record_ui_input_event(UiInputEventInput {
            session_id: cmd_session_id.clone(),
            route: route(),
            title: Some("cmd".to_string()),
            launch: Some(ShellLaunchSpec::new(r"C:\Windows\System32\cmd.exe")),
            pane_id: cmd_pane_id,
            data: "dir\r".to_string(),
            is_paste: false,
            source_event_id: Some("cmd-submit".to_string()),
            rows: None,
            cols: None,
            shell_kind: None,
        })
        .expect("cmd input should persist");
    store
        .record_ui_input_event(UiInputEventInput {
            session_id: powershell_session_id.clone(),
            route: route(),
            title: Some("powershell".to_string()),
            launch: Some(ShellLaunchSpec::new(
                r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe",
            )),
            pane_id: powershell_pane_id,
            data: "Get-Location\r".to_string(),
            is_paste: false,
            source_event_id: Some("powershell-submit".to_string()),
            rows: None,
            cols: None,
            shell_kind: None,
        })
        .expect("powershell input should persist");

    let mut connection = store.connection().expect("connection should open");
    let cmd_shell = terminal_command_history_entries::table
        .filter(terminal_command_history_entries::session_id.eq(Some(cmd_session_id)))
        .select(terminal_command_history_entries::shell_kind)
        .first::<Option<String>>(&mut connection)
        .expect("cmd history should load");
    let powershell_shell = terminal_command_history_entries::table
        .filter(
            terminal_command_history_entries::session_id.eq(Some(powershell_session_id.clone())),
        )
        .select(terminal_command_history_entries::shell_kind)
        .first::<Option<String>>(&mut connection)
        .expect("powershell history should load");
    let powershell_metadata = terminal_command_blocks::table
        .filter(terminal_command_blocks::session_id.eq(powershell_session_id))
        .select(terminal_command_blocks::metadata_json)
        .first::<Option<String>>(&mut connection)
        .expect("powershell command block metadata should load");
    let metadata: Value = serde_json::from_str(
        powershell_metadata.as_deref().expect("powershell command metadata should exist"),
    )
    .expect("powershell command metadata should be json");

    assert_eq!(cmd_shell.as_deref(), Some("cmd"));
    assert_eq!(powershell_shell.as_deref(), Some("powershell"));
    assert_eq!(metadata["shell_profile"]["shell_kind"], "powershell");
    assert_eq!(metadata["shell_profile"]["windows_profile"], true);
    assert_eq!(metadata["shell_profile"]["command_boundary_confidence"], "high");
}

#[test]
fn private_mode_suppresses_raw_output_and_command_history() {
    let store = test_store("private-mode");
    let session_id = store
        .create_session(SessionInput {
            id: None,
            route: route(),
            title: Some("private shell".to_string()),
            launch: None,
            source: Some("test".to_string()),
            durability_profile: None,
            retention_policy_id: None,
            private_mode: true,
            metadata: None,
        })
        .expect("private session should persist");
    let pane_id =
        store.create_pane(PaneInput::new(session_id.clone(), 24, 80)).expect("pane should persist");

    let output = store.record_terminal_output_event(TerminalOutputEventInput {
        session_id: session_id.clone(),
        route: route(),
        title: Some("private shell".to_string()),
        launch: None,
        pane_id: pane_id.clone(),
        tab_id: None,
        payload: b"secret-token-output\r\n".to_vec(),
        rows: Some(24),
        cols: Some(80),
        source_sequence: Some(1),
        occurred_at_ms: None,
        capture_semantics: Some("raw_vt_stream".to_string()),
    });
    let command = store.record_ui_input_event(UiInputEventInput {
        session_id: session_id.clone(),
        route: route(),
        title: Some("private shell".to_string()),
        launch: None,
        pane_id: pane_id.clone(),
        data: "echo secret-token-input\r".to_string(),
        is_paste: false,
        source_event_id: Some("private-submit".to_string()),
        rows: Some(24),
        cols: Some(80),
        shell_kind: Some("cmd".to_string()),
    });

    assert!(
        matches!(output, Err(TerminalPersistenceV2Error::InvalidData(message)) if message.contains("private mode suppresses durable terminal output capture"))
    );
    assert!(
        matches!(command, Err(TerminalPersistenceV2Error::InvalidData(message)) if message.contains("private mode suppresses durable ui input history"))
    );

    let segments = store
        .list_stream_segments(&session_id, &pane_id, 1, 10)
        .expect("segment query should succeed");
    let history =
        store.list_command_history(Some(&session_id), 10).expect("history query should load");
    let mut connection = store.connection().expect("connection should open");
    let private_mode = terminal_sessions::table
        .filter(terminal_sessions::id.eq(&session_id))
        .select(terminal_sessions::private_mode)
        .first::<i32>(&mut connection)
        .expect("session should load");

    assert_eq!(private_mode, 1);
    assert!(segments.is_empty());
    assert!(history.is_empty());
}

#[test]
fn dedupes_retried_ui_input_by_client_event_id() {
    let store = test_store("ui-input-retry");
    let session_id = Uuid::new_v4().to_string();
    let pane_id = Uuid::new_v4().to_string();
    let input = UiInputEventInput {
        session_id: session_id.clone(),
        route: route(),
        title: Some("shell".to_string()),
        launch: None,
        pane_id: pane_id.clone(),
        data: "git status\r".to_string(),
        is_paste: false,
        source_event_id: Some("browser-submit-1".to_string()),
        rows: None,
        cols: None,
        shell_kind: Some("cmd".to_string()),
    };

    store.record_ui_input_event(input.clone()).expect("first ui input should persist");
    store.record_ui_input_event(input).expect("retry should be deduped");

    let history =
        store.list_command_history(Some(&session_id), 10).expect("command history should load");
    let mut connection = store.connection().expect("connection should open");
    let event_count = terminal_journal_events::table
        .filter(terminal_journal_events::session_id.eq(&session_id))
        .filter(terminal_journal_events::pane_id.eq(Some(pane_id.clone())))
        .count()
        .get_result::<i64>(&mut connection)
        .expect("journal count should load");
    let command_block_count = terminal_command_blocks::table
        .filter(terminal_command_blocks::session_id.eq(&session_id))
        .filter(terminal_command_blocks::pane_id.eq(&pane_id))
        .count()
        .get_result::<i64>(&mut connection)
        .expect("command block count should load");
    let receipt_count = terminal_capture_receipts::table
        .filter(terminal_capture_receipts::session_id.eq(&session_id))
        .count()
        .get_result::<i64>(&mut connection)
        .expect("receipt count should load");

    assert_eq!(history.len(), 1);
    assert_eq!(history[0].display_text, "git status");
    assert_eq!(history[0].use_count, 1);
    assert_eq!(event_count, 1);
    assert_eq!(command_block_count, 1);
    assert_eq!(receipt_count, 1);
}

#[test]
fn rejects_ui_input_retry_with_same_client_event_id_and_different_payload() {
    let store = test_store("ui-input-retry-conflict");
    let session_id = Uuid::new_v4().to_string();
    let pane_id = Uuid::new_v4().to_string();
    let input = UiInputEventInput {
        session_id: session_id.clone(),
        route: route(),
        title: Some("shell".to_string()),
        launch: None,
        pane_id: pane_id.clone(),
        data: "git status\r".to_string(),
        is_paste: false,
        source_event_id: Some("browser-submit-2".to_string()),
        rows: None,
        cols: None,
        shell_kind: Some("cmd".to_string()),
    };
    store.record_ui_input_event(input.clone()).expect("first ui input should persist");

    let mut conflicting = input;
    conflicting.data = "git branch\r".to_string();
    let error =
        store.record_ui_input_event(conflicting).expect_err("conflicting retry should fail");
    let history =
        store.list_command_history(Some(&session_id), 10).expect("command history should load");

    assert!(
        matches!(error, TerminalPersistenceV2Error::InvalidData(message) if message.contains("payload hash mismatch"))
    );
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].display_text, "git status");
}
