use super::super::{prelude::*, support::*};

#[cfg(any(unix, windows))]
#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_smoke_persists_command_and_output_history_across_daemon_restart() {
    let store_path = unique_sqlite_path("bootstrap-v2-history-restart");
    let fixture = daemon_fixture_with_daemon(
        "bootstrap-v2-history-restart-a",
        TerminalDaemon::with_persistence(
            SqliteSessionStore::open(&store_path).expect("isolated sqlite store should open"),
        ),
    )
    .expect("fixture should start");
    let created = fixture
        .client
        .create_session(
            BackendKind::Native,
            CreateSessionSpec {
                title: Some("history-shell".to_string()),
                launch: Some(cat_launch_spec()),
            },
        )
        .await
        .expect("create_session should succeed");
    let topology = fixture
        .client
        .topology_snapshot(created.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let pane_id = topology.tabs[0].focused_pane.expect("focused pane should exist");

    wait_for_screen_line(&fixture, created.session.session_id, pane_id, "ready").await;
    let marker = format!("TERMINAL_HISTORY_RESTART_{}", created.session.session_id.0.simple());
    let command = format!("echo {marker}");
    fixture
        .client
        .dispatch(
            created.session.session_id,
            MuxCommand::SendInput(SendInputSpec {
                pane_id,
                data: submitted_input(&command),
                client_event_id: None,
            }),
        )
        .await
        .expect("send input should succeed");
    wait_for_screen_line(&fixture, created.session.session_id, pane_id, &marker).await;

    let before_restart_entry = wait_for_command_history_entry(
        &fixture,
        Some(created.session.session_id),
        20,
        "command history before daemon restart",
        |entry| {
            entry.session_id == Some(created.session.session_id)
                && entry.pane_id == Some(pane_id)
                && entry.display_text == command
        },
    )
    .await;
    let before_restart_pane_history = wait_for_pane_history_payload(
        &fixture,
        created.session.session_id,
        pane_id,
        marker.as_bytes(),
    )
    .await;
    fixture
        .client
        .dispatch(created.session.session_id, MuxCommand::SaveSession)
        .await
        .expect("save session should succeed");
    let saved = fixture
        .client
        .saved_session(created.session.session_id)
        .await
        .expect("saved_session should succeed");
    let saved_v2 = saved
        .session
        .restore_semantics_v2
        .as_ref()
        .expect("saved session should expose v2 restore semantics");

    assert_eq!(before_restart_entry.display_text, command);
    assert_eq!(before_restart_entry.use_count, 1);
    assert_eq!(before_restart_pane_history.session_id, created.session.session_id);
    assert_eq!(before_restart_pane_history.pane_id, pane_id);
    assert!(!before_restart_pane_history.segments.is_empty());
    assert!(before_restart_pane_history.total_payload_bytes > 0);
    assert_ne!(
        before_restart_pane_history.replay_strategy,
        terminal_protocol::PaneHistoryReplayStrategy::Empty
    );
    assert_eq!(saved_v2.source_session_id, created.session.session_id);
    assert_eq!(saved_v2.restored_session_id, None);
    assert_ne!(saved_v2.history_replay_state, terminal_protocol::HistoryReplayState::NotAvailable);
    assert!(!saved_v2.evidence_refs.is_empty());

    fixture.shutdown().await.expect("fixture should stop cleanly");

    let restarted = daemon_fixture_with_daemon(
        "bootstrap-v2-history-restart-b",
        TerminalDaemon::with_persistence(
            SqliteSessionStore::open(&store_path).expect("isolated sqlite store should reopen"),
        ),
    )
    .expect("restarted fixture should start");
    let after_restart_entry = wait_for_command_history_entry(
        &restarted,
        Some(created.session.session_id),
        20,
        "command history after daemon restart",
        |entry| {
            entry.session_id == Some(created.session.session_id)
                && entry.pane_id == Some(pane_id)
                && entry.display_text == command
        },
    )
    .await;
    let after_restart_pane_history = wait_for_pane_history_payload(
        &restarted,
        created.session.session_id,
        pane_id,
        marker.as_bytes(),
    )
    .await;
    let reloaded = restarted
        .client
        .saved_session(created.session.session_id)
        .await
        .expect("saved session should reload after daemon restart");

    assert_eq!(after_restart_entry.display_text, command);
    assert_eq!(after_restart_entry.use_count, 1);
    assert!(!after_restart_pane_history.segments.is_empty());
    assert_eq!(
        after_restart_pane_history.replay_strategy,
        before_restart_pane_history.replay_strategy
    );
    assert!(reloaded.session.restore_semantics_v2.is_some());

    restarted.shutdown().await.expect("restarted fixture should stop cleanly");
}

#[cfg(any(unix, windows))]
#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_smoke_scopes_command_history_by_session() {
    let fixture = daemon_fixture_with_daemon(
        "bootstrap-v2-command-history-scope",
        isolated_daemon("bootstrap-v2-command-history-scope"),
    )
    .expect("fixture should start");
    let first = create_session_with_command_history_marker(&fixture, "first").await;
    let second = create_session_with_command_history_marker(&fixture, "second").await;

    let first_history = fixture
        .client
        .command_history(Some(first.session_id), Some(20))
        .await
        .expect("first session command history should load");
    let second_history = fixture
        .client
        .command_history(Some(second.session_id), Some(20))
        .await
        .expect("second session command history should load");
    let global_history = fixture
        .client
        .command_history(None, Some(20))
        .await
        .expect("global command history should load");
    let limited_global_history = fixture
        .client
        .command_history(None, Some(1))
        .await
        .expect("limited global command history should load");

    assert!(first_history.entries.iter().any(|entry| entry.display_text == first.command));
    assert!(!first_history.entries.iter().any(|entry| entry.display_text == second.command));
    assert!(second_history.entries.iter().any(|entry| entry.display_text == second.command));
    assert!(!second_history.entries.iter().any(|entry| entry.display_text == first.command));
    assert!(global_history.entries.iter().any(|entry| entry.display_text == first.command));
    assert!(global_history.entries.iter().any(|entry| entry.display_text == second.command));
    assert_eq!(limited_global_history.entries.len(), 1);

    fixture.shutdown().await.expect("fixture should stop cleanly");
}

#[cfg(any(unix, windows))]
struct CommandHistoryMarker {
    session_id: SessionId,
    command: String,
}

#[cfg(any(unix, windows))]
async fn create_session_with_command_history_marker(
    fixture: &terminal_testing::DaemonFixture,
    label: &str,
) -> CommandHistoryMarker {
    let created = fixture
        .client
        .create_session(
            BackendKind::Native,
            CreateSessionSpec {
                title: Some(format!("history-{label}")),
                launch: Some(cat_launch_spec()),
            },
        )
        .await
        .expect("create_session should succeed");
    let topology = fixture
        .client
        .topology_snapshot(created.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let pane_id = topology.tabs[0].focused_pane.expect("focused pane should exist");

    wait_for_screen_line(fixture, created.session.session_id, pane_id, "ready").await;
    let marker =
        format!("TERMINAL_HISTORY_SCOPE_{label}_{}", created.session.session_id.0.simple());
    let command = format!("echo {marker}");
    fixture
        .client
        .dispatch(
            created.session.session_id,
            MuxCommand::SendInput(SendInputSpec {
                pane_id,
                data: submitted_input(&command),
                client_event_id: None,
            }),
        )
        .await
        .expect("send input should succeed");
    wait_for_screen_line(fixture, created.session.session_id, pane_id, &marker).await;
    wait_for_command_history_entry(
        fixture,
        Some(created.session.session_id),
        20,
        "scoped command history",
        |entry| {
            entry.session_id == Some(created.session.session_id)
                && entry.pane_id == Some(pane_id)
                && entry.display_text == command
        },
    )
    .await;

    CommandHistoryMarker { session_id: created.session.session_id, command }
}
