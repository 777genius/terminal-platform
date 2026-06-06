use super::super::{prelude::*, support::*};

#[cfg(any(unix, windows))]
#[ignore = "extended zellij stress coverage exceeds the portable CI latency budget"]
#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_smoke_handles_rapid_zellij_tab_focus_churn() {
    let session_name = unique_zellij_session_name("focus");
    let _zellij = ZellijSessionGuard::spawn(&session_name).expect("zellij session should start");
    let fixture = daemon_fixture("bootstrap-zellij-focus-churn").expect("fixture should start");
    let capabilities = fixture
        .client
        .backend_capabilities(BackendKind::Zellij)
        .await
        .expect("zellij capabilities should succeed");

    if !capabilities.capabilities.rendered_viewport_snapshot || !capabilities.capabilities.tab_focus
    {
        fixture.shutdown().await.expect("fixture should stop cleanly");
        return;
    }

    let candidate = wait_for_discovered_zellij_session(&fixture.client, &session_name).await;
    let imported = tokio::time::timeout(
        zellij_operation_timeout(),
        fixture.client.import_session(candidate.route, candidate.title),
    )
    .await
    .expect("import_session should not hang")
    .expect("zellij import should succeed");
    let mut subscription = fixture
        .client
        .open_subscription(imported.session.session_id, SubscriptionSpec::SessionTopology)
        .await
        .expect("topology subscription should open");

    let initial = must_recv_subscription_event(&mut subscription).await;
    assert!(matches!(initial, SubscriptionEvent::TopologySnapshot(_)));

    for title in ["focus-a", "focus-b"] {
        tokio::time::timeout(
            zellij_operation_timeout(),
            fixture.client.dispatch(
                imported.session.session_id,
                MuxCommand::NewTab(NewTabSpec { title: Some(title.to_string()) }),
            ),
        )
        .await
        .expect("zellij new_tab should not hang")
        .expect("zellij new_tab should succeed");
    }

    let initial_topology = wait_for_topology(
        &fixture,
        imported.session.session_id,
        |snapshot| {
            snapshot.tabs.len() >= 3
                && snapshot.tabs.iter().any(|tab| tab.title.as_deref() == Some("focus-a"))
                && snapshot.tabs.iter().any(|tab| tab.title.as_deref() == Some("focus-b"))
        },
        "zellij focus churn setup",
    )
    .await;
    let tab_ids: Vec<TabId> = initial_topology.tabs.iter().map(|tab| tab.tab_id).collect();
    let focus_sequence = vec![tab_ids[1], tab_ids[2], tab_ids[0], tab_ids[2]];
    let expected_final = *focus_sequence.last().expect("focus sequence should not be empty");

    for tab_id in &focus_sequence {
        tokio::time::timeout(
            zellij_operation_timeout(),
            fixture
                .client
                .dispatch(imported.session.session_id, MuxCommand::FocusTab { tab_id: *tab_id }),
        )
        .await
        .expect("zellij focus_tab should not hang")
        .expect("zellij focus_tab should succeed");
    }

    let final_topology = wait_for_topology(
        &fixture,
        imported.session.session_id,
        |snapshot| snapshot.focused_tab == Some(expected_final),
        "zellij focus churn final focus",
    )
    .await;
    let mut saw_final_event = false;

    for _ in 0..48 {
        let next = tokio::time::timeout(host_timeout(), subscription.recv())
            .await
            .expect("zellij churn subscription should not hang")
            .expect("zellij churn subscription should stay healthy");
        let Some(SubscriptionEvent::TopologySnapshot(snapshot)) = next else {
            continue;
        };
        if snapshot.focused_tab == Some(expected_final) {
            saw_final_event = true;
            break;
        }
    }

    let focused_tab = final_topology
        .tabs
        .iter()
        .find(|tab| tab.tab_id == expected_final)
        .expect("final focused tab should exist");
    let focused_pane = focused_tab
        .focused_pane
        .or_else(|| collect_pane_ids(&focused_tab.root).first().copied())
        .expect("focused pane should exist");
    let final_screen = fixture
        .client
        .screen_snapshot(imported.session.session_id, focused_pane)
        .await
        .expect("screen_snapshot should succeed");

    assert!(saw_final_event);
    assert_eq!(final_topology.focused_tab, Some(expected_final));
    assert_eq!(final_screen.pane_id, focused_pane);
    assert_eq!(final_screen.source, ProjectionSource::ZellijDumpSnapshot);

    subscription.close().await.expect("subscription should close cleanly");
    fixture.shutdown().await.expect("fixture should stop cleanly");
}
