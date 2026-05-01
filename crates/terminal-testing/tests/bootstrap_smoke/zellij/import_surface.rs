use super::super::{prelude::*, support::*};

#[cfg(any(unix, windows))]
#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_smoke_discovers_zellij_session_and_handles_import_surface() {
    let attempts = 3;
    let mut last_error = None;

    for attempt in 0..attempts {
        let run = tokio::spawn(async move {
            timeout(zellij_attempt_timeout(), async move {
                let session_name = unique_zellij_session_name("workspace");
                let _zellij =
                    ZellijSessionGuard::spawn(&session_name).expect("zellij session should start");
                let fixture =
                    daemon_fixture("bootstrap-zellij-discover").expect("fixture should start");
                let capabilities = fixture
                    .client
                    .backend_capabilities(BackendKind::Zellij)
                    .await
                    .expect("zellij capabilities should succeed");

                let candidate =
                    wait_for_discovered_zellij_session(&fixture.client, &session_name).await;
                assert_eq!(candidate.route.backend, BackendKind::Zellij);

                if !capabilities.capabilities.rendered_viewport_snapshot {
                    let error = tokio::time::timeout(
                        host_timeout(),
                        fixture
                            .client
                            .import_session(candidate.route.clone(), candidate.title.clone()),
                    )
                    .await
                    .expect("import_session should not hang")
                    .expect_err("legacy local zellij surface should reject imported attach");
                    let listed =
                        fixture.client.list_sessions().await.expect("list_sessions should succeed");

                    assert_eq!(error.code, "backend_unsupported");
                    assert!(error.message.contains("zellij 0.43.1"));
                    assert_eq!(error.degraded_reason, Some(DegradedModeReason::MissingCapability));
                    assert!(listed.sessions.is_empty());
                } else {
                    let imported = tokio::time::timeout(
                        zellij_operation_timeout(),
                        fixture
                            .client
                            .import_session(candidate.route.clone(), candidate.title.clone()),
                    )
                    .await
                    .expect("import_session should not hang")
                    .expect("rich zellij surface should import successfully");
                    let listed =
                        fixture.client.list_sessions().await.expect("list_sessions should succeed");
                    let topology = tokio::time::timeout(
                        host_timeout(),
                        fixture.client.topology_snapshot(imported.session.session_id),
                    )
                    .await
                    .expect("topology_snapshot should not hang")
                    .expect("topology_snapshot should succeed");
                    let focused_tab = topology
                        .tabs
                        .iter()
                        .find(|tab| Some(tab.tab_id) == topology.focused_tab)
                        .or_else(|| topology.tabs.first())
                        .expect("zellij topology should have tabs");
                    let focused_pane = focused_tab
                        .focused_pane
                        .or_else(|| collect_pane_ids(&focused_tab.root).first().copied())
                        .expect("focused zellij pane should exist");
                    let screen = tokio::time::timeout(
                        host_timeout(),
                        fixture.client.screen_snapshot(imported.session.session_id, focused_pane),
                    )
                    .await
                    .expect("screen_snapshot should not hang")
                    .expect("screen_snapshot should succeed");
                    let delta = tokio::time::timeout(
                        host_timeout(),
                        fixture.client.screen_delta(
                            imported.session.session_id,
                            focused_pane,
                            screen.sequence,
                        ),
                    )
                    .await
                    .expect("screen_delta should not hang")
                    .expect("screen_delta should succeed");
                    let mut topology_subscription = fixture
                        .client
                        .open_subscription(
                            imported.session.session_id,
                            SubscriptionSpec::SessionTopology,
                        )
                        .await
                        .expect("zellij topology subscription should open");
                    let mut pane_subscription = fixture
                        .client
                        .open_subscription(
                            imported.session.session_id,
                            SubscriptionSpec::PaneSurface { pane_id: focused_pane },
                        )
                        .await
                        .expect("zellij pane subscription should open");
                    let initial_topology =
                        tokio::time::timeout(host_timeout(), topology_subscription.recv())
                            .await
                            .expect("zellij topology subscription should not hang")
                            .expect("zellij topology subscription should stay healthy")
                            .expect("zellij topology subscription should emit initial event");
                    let initial_pane =
                        tokio::time::timeout(host_timeout(), pane_subscription.recv())
                            .await
                            .expect("zellij pane subscription should not hang")
                            .expect("zellij pane subscription should stay healthy")
                            .expect("zellij pane subscription should emit initial event");

                    assert_eq!(imported.session.route.backend, BackendKind::Zellij);
                    assert!(
                        listed
                            .sessions
                            .iter()
                            .any(|session| session.session_id == imported.session.session_id)
                    );
                    assert_eq!(topology.backend_kind, BackendKind::Zellij);
                    assert!(!topology.tabs.is_empty());
                    assert_eq!(screen.pane_id, focused_pane);
                    assert_eq!(screen.source, ProjectionSource::ZellijDumpSnapshot);
                    assert_zellij_delta_compatible_with_snapshot(&screen, &delta);
                    match initial_topology {
                        SubscriptionEvent::TopologySnapshot(snapshot) => {
                            assert_eq!(snapshot.session_id, imported.session.session_id);
                            assert_eq!(snapshot.backend_kind, BackendKind::Zellij);
                        }
                        other => panic!("unexpected initial zellij topology event: {other:?}"),
                    }
                    match initial_pane {
                        SubscriptionEvent::ScreenDelta(delta) => {
                            assert_eq!(delta.pane_id, focused_pane);
                            assert_eq!(delta.source, ProjectionSource::ZellijDumpSnapshot);
                            assert!(delta.full_replace.is_some());
                        }
                        other => panic!("unexpected initial zellij pane event: {other:?}"),
                    }

                    let initial_tab_count = topology.tabs.len();
                    let initial_focused_tab =
                        topology.focused_tab.expect("focused zellij tab should exist");

                    if capabilities.capabilities.pane_input_write {
                        let input_marker = format!(
                            "TERMINAL_ZELLIJ_INPUT_{}",
                            imported.session.session_id.0.simple()
                        );
                        let input_command = format!("echo {input_marker}");
                        let input = tokio::time::timeout(
                            zellij_operation_timeout(),
                            fixture.client.dispatch(
                                imported.session.session_id,
                                MuxCommand::SendInput(SendInputSpec {
                                    pane_id: focused_pane,
                                    data: submitted_input(&input_command),
                                    client_event_id: Some(format!(
                                        "zellij-input-{}",
                                        imported.session.session_id.0.simple()
                                    )),
                                }),
                            ),
                        )
                        .await
                        .expect("zellij send_input should not hang")
                        .expect("zellij send_input should succeed");
                        wait_for_screen_line(
                            &fixture,
                            imported.session.session_id,
                            focused_pane,
                            &input_marker,
                        )
                        .await;
                        let input_history = wait_for_command_history_entry(
                            &fixture,
                            Some(imported.session.session_id),
                            20,
                            "zellij imported command history",
                            |entry| {
                                entry.session_id == Some(imported.session.session_id)
                                    && entry.pane_id == Some(focused_pane)
                                    && entry.display_text == input_command
                            },
                        )
                        .await;

                        assert!(input.changed);
                        assert_eq!(input_history.display_text, input_command);
                    }

                    if capabilities.capabilities.pane_paste_write {
                        let paste_marker = format!(
                            "TERMINAL_ZELLIJ_PASTE_{}",
                            imported.session.session_id.0.simple()
                        );
                        let paste_command = format!("echo {paste_marker}");
                        let paste = tokio::time::timeout(
                            zellij_operation_timeout(),
                            fixture.client.dispatch(
                                imported.session.session_id,
                                MuxCommand::SendPaste(SendPasteSpec {
                                    pane_id: focused_pane,
                                    data: paste_command,
                                    client_event_id: Some(format!(
                                        "zellij-paste-{}",
                                        imported.session.session_id.0.simple()
                                    )),
                                }),
                            ),
                        )
                        .await
                        .expect("zellij send_paste should not hang")
                        .expect("zellij send_paste should succeed");
                        let paste_enter = tokio::time::timeout(
                            zellij_operation_timeout(),
                            fixture.client.dispatch(
                                imported.session.session_id,
                                MuxCommand::SendInput(SendInputSpec {
                                    pane_id: focused_pane,
                                    data: submitted_input(""),
                                    client_event_id: Some(format!(
                                        "zellij-paste-enter-{}",
                                        imported.session.session_id.0.simple()
                                    )),
                                }),
                            ),
                        )
                        .await
                        .expect("zellij paste enter should not hang")
                        .expect("zellij paste enter should succeed");
                        wait_for_screen_line(
                            &fixture,
                            imported.session.session_id,
                            focused_pane,
                            &paste_marker,
                        )
                        .await;

                        assert!(paste.changed);
                        assert!(paste_enter.changed);
                    }

                    let save_error = tokio::time::timeout(
                        host_timeout(),
                        fixture
                            .client
                            .dispatch(imported.session.session_id, MuxCommand::SaveSession),
                    )
                    .await
                    .expect("zellij save_session should not hang")
                    .expect_err("zellij imported save_session should be unsupported");
                    assert_eq!(save_error.code, "backend_unsupported");
                    assert_eq!(
                        save_error.degraded_reason,
                        Some(DegradedModeReason::UnsupportedByBackend)
                    );

                    let created = tokio::time::timeout(
                        zellij_operation_timeout(),
                        fixture.client.dispatch(
                            imported.session.session_id,
                            MuxCommand::NewTab(NewTabSpec { title: Some("logs-rich".to_string()) }),
                        ),
                    )
                    .await
                    .expect("zellij new_tab should not hang")
                    .expect("zellij new_tab should succeed");
                    let after_create = wait_for_topology(
                        &fixture,
                        imported.session.session_id,
                        |snapshot| {
                            snapshot.tabs.len() == initial_tab_count + 1
                                && snapshot
                                    .tabs
                                    .iter()
                                    .any(|tab| tab.title.as_deref() == Some("logs-rich"))
                        },
                        "zellij rich new tab topology",
                    )
                    .await;
                    let rich_tab_id = after_create
                        .tabs
                        .iter()
                        .find(|tab| tab.title.as_deref() == Some("logs-rich"))
                        .map(|tab| tab.tab_id)
                        .expect("created rich zellij tab should exist");

                    let renamed = tokio::time::timeout(
                        zellij_operation_timeout(),
                        fixture.client.dispatch(
                            imported.session.session_id,
                            MuxCommand::RenameTab {
                                tab_id: rich_tab_id,
                                title: "logs-rich-renamed".to_string(),
                            },
                        ),
                    )
                    .await
                    .expect("zellij rename_tab should not hang")
                    .expect("zellij rename_tab should succeed");
                    let after_rename = wait_for_topology(
                        &fixture,
                        imported.session.session_id,
                        |snapshot| {
                            snapshot.tabs.iter().any(|tab| {
                                tab.tab_id == rich_tab_id
                                    && tab.title.as_deref() == Some("logs-rich-renamed")
                            })
                        },
                        "zellij rich renamed tab topology",
                    )
                    .await;

                    let focused = if capabilities.capabilities.tab_focus {
                        let focused = tokio::time::timeout(
                            zellij_operation_timeout(),
                            fixture.client.dispatch(
                                imported.session.session_id,
                                MuxCommand::FocusTab { tab_id: initial_focused_tab },
                            ),
                        )
                        .await
                        .expect("zellij focus_tab should not hang")
                        .expect("zellij focus_tab should succeed");
                        let after_focus = wait_for_topology(
                            &fixture,
                            imported.session.session_id,
                            |snapshot| snapshot.focused_tab == Some(initial_focused_tab),
                            "zellij rich focus tab topology",
                        )
                        .await;
                        Some((focused, after_focus))
                    } else {
                        None
                    };

                    let closed = tokio::time::timeout(
                        zellij_operation_timeout(),
                        fixture.client.dispatch(
                            imported.session.session_id,
                            MuxCommand::CloseTab { tab_id: rich_tab_id },
                        ),
                    )
                    .await
                    .expect("zellij close_tab should not hang")
                    .expect("zellij close_tab should succeed");
                    let after_close = wait_for_topology(
                        &fixture,
                        imported.session.session_id,
                        |snapshot| {
                            snapshot.tabs.len() == initial_tab_count
                                && snapshot.tabs.iter().all(|tab| tab.tab_id != rich_tab_id)
                        },
                        "zellij rich close tab topology",
                    )
                    .await;

                    assert!(created.changed);
                    assert_eq!(after_create.tabs.len(), initial_tab_count + 1);
                    assert!(renamed.changed);
                    assert!(after_rename.tabs.iter().any(|tab| {
                        tab.tab_id == rich_tab_id
                            && tab.title.as_deref() == Some("logs-rich-renamed")
                    }));
                    if let Some((focused, after_focus)) = focused {
                        assert!(focused.changed);
                        assert_eq!(after_focus.focused_tab, Some(initial_focused_tab));
                    }
                    assert!(closed.changed);
                    assert_eq!(after_close.tabs.len(), initial_tab_count);

                    topology_subscription
                        .close()
                        .await
                        .expect("topology subscription should close cleanly");
                    pane_subscription
                        .close()
                        .await
                        .expect("pane subscription should close cleanly");
                }

                fixture.shutdown().await.expect("fixture should stop cleanly");
            })
            .await
            .expect("bootstrap zellij smoke attempt should complete within timeout");
        });

        match run.await {
            Ok(()) => return,
            Err(error) => {
                last_error = Some(format!("attempt {} failed: {error}", attempt + 1));
                sleep(Duration::from_millis(250)).await;
            }
        }
    }

    panic!(
        "bootstrap zellij import smoke failed after {attempts} attempts: {}",
        last_error.unwrap_or_else(|| "unknown failure".to_string())
    );
}
