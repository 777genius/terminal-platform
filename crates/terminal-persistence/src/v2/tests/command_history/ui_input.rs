use super::super::super::*;
use super::super::support::*;

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
