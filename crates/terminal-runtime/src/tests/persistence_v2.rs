use super::{fakes::*, prelude::*, support::*};

#[tokio::test(flavor = "multi_thread")]
async fn save_session_dual_writes_v2_visual_snapshot() {
    let path = unique_runtime_store_path("v2-save-session");
    let store = SqliteSessionStore::open(&path).expect("isolated sqlite store should open");
    let runtime = TerminalRuntime::with_persistence(
        BackendCatalog::new([Arc::new(FakeNativeBackend) as Arc<dyn MuxBackendPort>]),
        store,
    );
    let created = runtime
        .create_session(
            BackendKind::Native,
            CreateSessionSpec {
                title: Some("shell".to_string()),
                launch: Some(terminal_backend_api::ShellLaunchSpec::new("cmd.exe")),
            },
        )
        .await
        .expect("fake native session should create");

    runtime
        .dispatch(created.session_id, MuxCommand::SaveSession)
        .await
        .expect("save session should dual-write");

    let v2 = terminal_persistence::TerminalPersistenceV2::open_with_config(
        &path,
        terminal_persistence::TerminalPersistenceV2Config::test(),
    )
    .expect("v2 store should open");
    let plan = v2.restore_plan(&created.session_id.0.to_string()).expect("plan should load");

    assert_eq!(
        plan.guarantee_level,
        terminal_persistence::RestoreGuaranteeLevel::VisualSnapshotOnly
    );
    assert!(plan.latest_screen_snapshot_id.is_some());
    assert!(plan.latest_topology_snapshot_id.is_some());
}

#[tokio::test(flavor = "multi_thread")]
async fn dispatch_send_input_records_v2_command_history() {
    let path = unique_runtime_store_path("v2-ui-input");
    let store = SqliteSessionStore::open(&path).expect("isolated sqlite store should open");
    let runtime = TerminalRuntime::with_persistence(
        BackendCatalog::new([Arc::new(FakeNativeBackend) as Arc<dyn MuxBackendPort>]),
        store,
    );
    let created = runtime
        .create_session(
            BackendKind::Native,
            CreateSessionSpec {
                title: Some("shell".to_string()),
                launch: Some(terminal_backend_api::ShellLaunchSpec::new("cmd.exe")),
            },
        )
        .await
        .expect("fake native session should create");
    let topology =
        runtime.topology_snapshot(created.session_id).await.expect("topology should load");
    let pane_id = topology.tabs[0].focused_pane.expect("focused pane should exist");

    runtime
        .dispatch(
            created.session_id,
            MuxCommand::SendInput(SendInputSpec {
                pane_id,
                data: "git status\r".to_string(),
                client_event_id: None,
            }),
        )
        .await
        .expect("send input should dispatch");

    let history = wait_for_v2_command_history(&path, created.session_id)
        .await
        .expect("command history should be captured");

    assert_eq!(history.len(), 1);
    assert_eq!(history[0].display_text, "git status");
}

#[tokio::test(flavor = "multi_thread")]
async fn native_runtime_capture_persists_raw_output_to_v2() {
    let path = unique_runtime_store_path("v2-native-output");
    let store = SqliteSessionStore::open(&path).expect("isolated sqlite store should open");
    let runtime = TerminalRuntime::with_persistence(
        BackendCatalog::new([Arc::new(NativeBackend::default()) as Arc<dyn MuxBackendPort>]),
        store,
    );
    let created = runtime
        .create_session(
            BackendKind::Native,
            CreateSessionSpec {
                title: Some("capture-shell".to_string()),
                launch: Some(capture_shell_launch_spec()),
            },
        )
        .await
        .expect("native session should create");
    let topology =
        runtime.topology_snapshot(created.session_id).await.expect("topology should load");
    let pane_id = topology.tabs[0].focused_pane.expect("focused pane should exist");

    wait_for_v2_snapshot(&path, created.session_id)
        .await
        .expect("capture task should write initial snapshot");
    wait_for_runtime_screen_line(&runtime, created.session_id, pane_id, "ready")
        .await
        .expect("native shell should become ready");

    let marker = format!("TERMINAL_PERSISTENCE_V2_CAPTURE_{}", created.session_id.0.simple());
    runtime
        .dispatch(
            created.session_id,
            MuxCommand::SendInput(SendInputSpec {
                pane_id,
                data: capture_shell_echo_input(&marker),
                client_event_id: None,
            }),
        )
        .await
        .expect("marker command should dispatch");
    wait_for_runtime_screen_line(&runtime, created.session_id, pane_id, &marker)
        .await
        .expect("native shell should render marker output");

    let payload = wait_for_v2_payload(&path, created.session_id, pane_id, marker.as_bytes())
        .await
        .expect("v2 raw output capture should persist marker");

    assert!(payload.windows(marker.len()).any(|window| window == marker.as_bytes()));

    runtime
        .dispatch(
            created.session_id,
            MuxCommand::SplitPane(SplitPaneSpec { pane_id, direction: SplitDirection::Vertical }),
        )
        .await
        .expect("split pane should dispatch through runtime");
    let after_split = wait_for_runtime_topology(&runtime, created.session_id, |topology| {
        topology.tabs.first().map_or(false, |tab| collect_test_pane_ids(&tab.root).len() >= 2)
    })
    .await
    .expect("split topology should be observed");
    let split_pane = collect_test_pane_ids(&after_split.tabs[0].root)
        .into_iter()
        .find(|candidate| *candidate != pane_id)
        .expect("split should create a second pane");
    runtime
        .dispatch(created.session_id, MuxCommand::ClosePane { pane_id: split_pane })
        .await
        .expect("close pane should dispatch through runtime");
    let after_close_pane = wait_for_runtime_topology(&runtime, created.session_id, |topology| {
        topology.tabs.first().map_or(false, |tab| collect_test_pane_ids(&tab.root) == vec![pane_id])
    })
    .await
    .expect("closed pane topology should be observed");
    let original_tab_id = after_close_pane.tabs[0].tab_id;

    runtime
        .dispatch(
            created.session_id,
            MuxCommand::NewTab(NewTabSpec { title: Some("Logs".to_string()) }),
        )
        .await
        .expect("new tab should dispatch through runtime");
    let after_new_tab = wait_for_runtime_topology(&runtime, created.session_id, |topology| {
        topology.tabs.len() >= 2
    })
    .await
    .expect("new tab topology should be observed");
    let new_tab_id = after_new_tab
        .tabs
        .iter()
        .map(|tab| tab.tab_id)
        .find(|tab_id| *tab_id != original_tab_id)
        .expect("new tab id should be present");
    runtime
        .dispatch(created.session_id, MuxCommand::FocusTab { tab_id: original_tab_id })
        .await
        .expect("focus original tab should dispatch through runtime");
    runtime
        .dispatch(created.session_id, MuxCommand::CloseTab { tab_id: new_tab_id })
        .await
        .expect("close tab should dispatch through runtime");
    wait_for_runtime_topology(&runtime, created.session_id, |topology| {
        topology.tabs.len() == 1 && topology.focused_tab == Some(original_tab_id)
    })
    .await
    .expect("closed tab topology should be observed");
    runtime
        .dispatch(
            created.session_id,
            MuxCommand::RenameTab { tab_id: original_tab_id, title: "Smoke Workspace".into() },
        )
        .await
        .expect("rename tab should dispatch through runtime");

    let save_marker = format!("TERMINAL_PERSISTENCE_V2_SAVE_{}", created.session_id.0.simple());
    runtime
        .dispatch(
            created.session_id,
            MuxCommand::SendInput(SendInputSpec {
                pane_id,
                data: capture_shell_echo_input(&save_marker),
                client_event_id: None,
            }),
        )
        .await
        .expect("save marker command should dispatch");
    wait_for_runtime_screen_line(&runtime, created.session_id, pane_id, &save_marker)
        .await
        .expect("native shell should render save marker output");

    runtime
        .dispatch(created.session_id, MuxCommand::SaveSession)
        .await
        .expect("save session should preserve a healthy v2 restore plan");
    let plan = wait_for_v2_restore_plan(&path, created.session_id, |plan| {
        plan.latest_restore_drill_status.as_deref() == Some("passed")
            && plan.evidence.iter().any(|evidence| {
                evidence.kind == "backend_capture_semantics" && evidence.value == "raw_vt_stream"
            })
    })
    .await
    .expect("v2 restore plan should include runtime raw capture capability evidence");
    assert_eq!(
        plan.latest_restore_drill_status.as_deref(),
        Some("passed"),
        "unexpected restore plan after native save: {plan:?}"
    );
    assert_eq!(
        plan.guarantee_level,
        terminal_persistence::RestoreGuaranteeLevel::RawStreamReplay,
        "raw capture with a passed restore drill should promote restore guarantee: {plan:?}"
    );
    assert!(plan.evidence.iter().any(|evidence| {
        evidence.kind == "backend_capture_strategy" && evidence.value == "raw_stream"
    }));
}
