use super::{prelude::*, support::*};

#[cfg(any(unix, windows))]
#[tokio::test(flavor = "multi_thread")]
async fn discovers_zellij_sessions_and_handles_import_surface_through_node_surface() {
    let attempts = if cfg!(windows) { 1 } else { 3 };
    let mut last_error = None;

    for attempt in 0..attempts {
        let run = tokio::spawn(async move {
            timeout(zellij_attempt_timeout(), async move {
                let session_name = unique_zellij_session_name("workspace");
                let _zellij =
                    ZellijSessionGuard::spawn(&session_name).expect("zellij session should start");
                let fixture = daemon_fixture("terminal-node-zellij").expect("fixture should start");
                let node = NodeHostClient::new(fixture.client.address().clone());
                let zellij_capabilities = node
                    .backend_capabilities(NodeBackendKind::Zellij)
                    .await
                    .expect("zellij capabilities should succeed");

                let candidate = wait_for_discovered_zellij_session(&node, &session_name).await;

                assert_eq!(candidate.route.backend, NodeBackendKind::Zellij);

                if !zellij_capabilities.capabilities.rendered_viewport_snapshot {
                    let error = timeout(
                        extended_timeout(),
                        node.import_session(&candidate.route, candidate.title.clone()),
                    )
                    .await
                    .expect("import_session should not hang")
                    .expect_err("legacy zellij surface should reject imported attach");

                    assert_eq!(error.code, "backend_unsupported");
                    assert_eq!(error.degraded_reason, Some(DegradedModeReason::MissingCapability));
                    assert!(error.message.contains("zellij"));
                } else {
                    let imported = timeout(
                        zellij_operation_timeout(),
                        node.import_session(&candidate.route, candidate.title.clone()),
                    )
                    .await
                    .expect("import_session should not hang")
                    .expect("rich zellij surface should import successfully");
                    let topology =
                        timeout(extended_timeout(), node.topology_snapshot(&imported.session_id))
                            .await
                            .expect("topology_snapshot should not hang")
                            .expect("topology_snapshot should succeed");
                    let focused_tab = topology
                        .tabs
                        .iter()
                        .find(|tab| Some(tab.tab_id.as_str()) == topology.focused_tab.as_deref())
                        .or_else(|| topology.tabs.first())
                        .expect("zellij topology should have tabs");
                    let focused_pane = focused_tab
                        .focused_pane
                        .clone()
                        .or_else(|| first_node_pane_id(&focused_tab.root))
                        .expect("focused zellij pane should exist");
                    let screen = timeout(
                        extended_timeout(),
                        node.screen_snapshot(&imported.session_id, &focused_pane),
                    )
                    .await
                    .expect("screen_snapshot should not hang")
                    .expect("screen_snapshot should succeed");
                    let delta = timeout(
                        extended_timeout(),
                        node.screen_delta(&imported.session_id, &focused_pane, screen.sequence),
                    )
                    .await
                    .expect("screen_delta should not hang")
                    .expect("screen_delta should succeed");
                    let topology_subscription = node
                        .open_subscription(
                            &imported.session_id,
                            &NodeSubscriptionSpec::SessionTopology,
                        )
                        .await
                        .expect("zellij topology subscription should open");
                    let pane_subscription = node
                        .open_subscription(
                            &imported.session_id,
                            &NodeSubscriptionSpec::PaneSurface { pane_id: focused_pane.clone() },
                        )
                        .await
                        .expect("zellij pane subscription should open");
                    let initial_topology =
                        timeout(extended_timeout(), topology_subscription.next_event())
                            .await
                            .expect("zellij topology subscription should not hang")
                            .expect("zellij topology subscription should stay healthy")
                            .expect("zellij topology subscription should emit initial event");
                    let initial_pane = timeout(extended_timeout(), pane_subscription.next_event())
                        .await
                        .expect("zellij pane subscription should not hang")
                        .expect("zellij pane subscription should stay healthy")
                        .expect("zellij pane subscription should emit initial event");

                    assert_eq!(imported.route.backend, NodeBackendKind::Zellij);
                    assert_eq!(topology.backend_kind, NodeBackendKind::Zellij);
                    assert!(!topology.tabs.is_empty());
                    assert_eq!(screen.pane_id, focused_pane);
                    assert_eq!(screen.source, NodeProjectionSource::ZellijDumpSnapshot);
                    assert_zellij_delta_compatible_with_snapshot(&screen, &delta);
                    match initial_topology {
                        NodeSubscriptionEvent::TopologySnapshot(snapshot) => {
                            assert_eq!(snapshot.session_id, imported.session_id);
                            assert_eq!(snapshot.backend_kind, NodeBackendKind::Zellij);
                        }
                        NodeSubscriptionEvent::SessionHealthSnapshot(health) => {
                            panic!("unexpected initial zellij topology health event: {health:?}");
                        }
                        other => panic!("unexpected initial zellij topology event: {other:?}"),
                    }
                    match initial_pane {
                        NodeSubscriptionEvent::ScreenDelta(delta) => {
                            assert_eq!(delta.pane_id, focused_pane);
                            assert_eq!(delta.source, NodeProjectionSource::ZellijDumpSnapshot);
                            assert!(delta.full_replace.is_some());
                        }
                        NodeSubscriptionEvent::SessionHealthSnapshot(health) => {
                            panic!("unexpected initial zellij pane health event: {health:?}");
                        }
                        other => panic!("unexpected initial zellij pane event: {other:?}"),
                    }

                    let initial_tab_count = topology.tabs.len();
                    let initial_focused_tab =
                        topology.focused_tab.clone().expect("focused zellij tab should exist");

                    let created = timeout(
                        zellij_operation_timeout(),
                        node.dispatch_mux_command(
                            &imported.session_id,
                            &NodeMuxCommand::NewTab(NodeNewTabCommand {
                                title: Some("logs-rich".to_string()),
                            }),
                        ),
                    )
                    .await
                    .expect("zellij new_tab should not hang")
                    .expect("zellij new_tab should succeed");
                    let after_create = wait_for_topology_state(
                        &node,
                        &imported.session_id,
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
                        .map(|tab| tab.tab_id.clone())
                        .expect("created rich zellij tab should exist");

                    let renamed = timeout(
                        zellij_operation_timeout(),
                        node.dispatch_mux_command(
                            &imported.session_id,
                            &NodeMuxCommand::RenameTab(NodeRenameTabCommand {
                                tab_id: rich_tab_id.clone(),
                                title: "logs-rich-renamed".to_string(),
                            }),
                        ),
                    )
                    .await
                    .expect("zellij rename_tab should not hang")
                    .expect("zellij rename_tab should succeed");
                    let after_rename = wait_for_topology_state(
                        &node,
                        &imported.session_id,
                        |snapshot| {
                            snapshot.tabs.iter().any(|tab| {
                                tab.tab_id == rich_tab_id
                                    && tab.title.as_deref() == Some("logs-rich-renamed")
                            })
                        },
                        "zellij rich renamed tab topology",
                    )
                    .await;

                    let focused = if zellij_capabilities.capabilities.tab_focus {
                        let focused = timeout(
                            zellij_operation_timeout(),
                            node.dispatch_mux_command(
                                &imported.session_id,
                                &NodeMuxCommand::FocusTab { tab_id: initial_focused_tab.clone() },
                            ),
                        )
                        .await
                        .expect("zellij focus_tab should not hang")
                        .expect("zellij focus_tab should succeed");
                        let after_focus = wait_for_topology_state(
                            &node,
                            &imported.session_id,
                            |snapshot| {
                                snapshot.focused_tab.as_deref()
                                    == Some(initial_focused_tab.as_str())
                            },
                            "zellij rich focus tab topology",
                        )
                        .await;
                        Some((focused, after_focus))
                    } else {
                        None
                    };

                    let closed = timeout(
                        zellij_operation_timeout(),
                        node.dispatch_mux_command(
                            &imported.session_id,
                            &NodeMuxCommand::CloseTab { tab_id: rich_tab_id.clone() },
                        ),
                    )
                    .await
                    .expect("zellij close_tab should not hang")
                    .expect("zellij close_tab should succeed");
                    let after_close = wait_for_topology_state(
                        &node,
                        &imported.session_id,
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
                        assert_eq!(
                            after_focus.focused_tab.as_deref(),
                            Some(initial_focused_tab.as_str())
                        );
                    }
                    assert!(closed.changed);
                    assert_eq!(after_close.tabs.len(), initial_tab_count);

                    topology_subscription.close().await;
                    pane_subscription.close().await;
                }

                fixture.shutdown().await.expect("fixture should stop cleanly");
            })
            .await
            .expect("zellij node smoke attempt should complete within timeout");
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
        "node zellij import smoke failed after {attempts} attempts: {}",
        last_error.unwrap_or_else(|| "unknown failure".to_string())
    );
}
