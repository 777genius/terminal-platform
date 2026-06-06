use super::super::{prelude::*, support::*};

#[cfg(any(unix, windows))]
#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_smoke_handles_rapid_native_tab_focus_churn() {
    let fixture = daemon_fixture("bootstrap-native-focus-churn").expect("fixture should start");
    let created = fixture
        .client
        .create_session(
            BackendKind::Native,
            CreateSessionSpec { title: Some("shell".to_string()), launch: Some(cat_launch_spec()) },
        )
        .await
        .expect("create_session should succeed");
    let mut subscription = fixture
        .client
        .open_subscription(created.session.session_id, SubscriptionSpec::SessionTopology)
        .await
        .expect("topology subscription should open");

    let initial = must_recv_subscription_event(&mut subscription).await;
    assert!(matches!(initial, SubscriptionEvent::TopologySnapshot(_)));

    for title in ["logs-a", "logs-b"] {
        fixture
            .client
            .dispatch(
                created.session.session_id,
                MuxCommand::NewTab(NewTabSpec { title: Some(title.to_string()) }),
            )
            .await
            .expect("new tab should succeed");
    }

    let initial_topology = wait_for_topology(
        &fixture,
        created.session.session_id,
        |snapshot| snapshot.tabs.len() == 3,
        "native tab churn setup",
    )
    .await;
    let tab_ids: Vec<TabId> = initial_topology.tabs.iter().map(|tab| tab.tab_id).collect();
    let focus_sequence = vec![
        tab_ids[1], tab_ids[2], tab_ids[0], tab_ids[2], tab_ids[1], tab_ids[0], tab_ids[2],
        tab_ids[1], tab_ids[0], tab_ids[2],
    ];
    let expected_final = *focus_sequence.last().expect("focus sequence should not be empty");

    for tab_id in &focus_sequence {
        fixture
            .client
            .dispatch(created.session.session_id, MuxCommand::FocusTab { tab_id: *tab_id })
            .await
            .expect("focus tab should succeed");
    }

    let final_topology = wait_for_topology(
        &fixture,
        created.session.session_id,
        |snapshot| snapshot.focused_tab == Some(expected_final),
        "native tab churn final focus",
    )
    .await;
    let mut saw_final_event = false;

    for _ in 0..48 {
        let next = tokio::time::timeout(host_timeout(), subscription.recv())
            .await
            .expect("native churn subscription should not hang")
            .expect("native churn subscription should stay healthy");
        let Some(SubscriptionEvent::TopologySnapshot(snapshot)) = next else {
            continue;
        };
        if snapshot.focused_tab == Some(expected_final) {
            saw_final_event = true;
            break;
        }
    }

    assert_eq!(final_topology.tabs.len(), 3);
    assert_eq!(final_topology.focused_tab, Some(expected_final));
    assert!(saw_final_event);

    subscription.close().await.expect("subscription should close cleanly");
    fixture.shutdown().await.expect("fixture should stop cleanly");
}
