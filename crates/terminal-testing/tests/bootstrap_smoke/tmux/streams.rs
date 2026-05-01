use super::super::{prelude::*, support::*};

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_smoke_streams_tmux_topology_updates() {
    let socket_name = unique_tmux_socket_name("bootstrap-tmux-topology");
    let session_name = unique_tmux_session_name("workspace");
    let _tmux =
        TmuxServerGuard::spawn(&socket_name, &session_name).expect("tmux test server should start");
    let fixture =
        daemon_fixture_with_daemon("bootstrap-tmux-topology-sub", tmux_daemon(&socket_name))
            .expect("fixture should start");

    let discovered = fixture
        .client
        .discover_sessions(BackendKind::Tmux)
        .await
        .expect("discover_sessions should succeed");
    let imported = fixture
        .client
        .import_session(discovered.sessions[0].route.clone(), discovered.sessions[0].title.clone())
        .await
        .expect("import_session should succeed");
    let mut subscription = fixture
        .client
        .open_subscription(imported.session.session_id, SubscriptionSpec::SessionTopology)
        .await
        .expect("subscription should open");

    let initial = must_recv_subscription_event(&mut subscription).await;
    let initial = match initial {
        SubscriptionEvent::TopologySnapshot(snapshot) => snapshot,
        other => panic!("unexpected initial event: {other:?}"),
    };

    run_tmux(
        &socket_name,
        &[
            "new-window",
            "-d",
            "-t",
            &session_name,
            "-n",
            "metrics",
            "sh",
            "-lc",
            "printf 'metrics ready\\n'; exec cat",
        ],
    )
    .expect("tmux new-window should succeed");

    let updated = must_recv_subscription_event(&mut subscription).await;
    let mut updated = match updated {
        SubscriptionEvent::TopologySnapshot(snapshot) => snapshot,
        other => panic!("unexpected topology event: {other:?}"),
    };
    while updated.tabs.len() != 3 {
        let next = must_recv_subscription_event(&mut subscription).await;
        updated = match next {
            SubscriptionEvent::TopologySnapshot(snapshot) => snapshot,
            other => panic!("unexpected topology event: {other:?}"),
        };
    }

    assert_eq!(initial.tabs.len(), 2);
    assert_eq!(updated.tabs.len(), 3);

    fixture.shutdown().await.expect("fixture should stop cleanly");
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_smoke_streams_tmux_pane_surface_updates() {
    let socket_name = unique_tmux_socket_name("bootstrap-tmux-pane");
    let session_name = unique_tmux_session_name("workspace");
    let _tmux =
        TmuxServerGuard::spawn(&socket_name, &session_name).expect("tmux test server should start");
    let fixture = daemon_fixture_with_daemon("bootstrap-tmux-pane-sub", tmux_daemon(&socket_name))
        .expect("fixture should start");

    let discovered = fixture
        .client
        .discover_sessions(BackendKind::Tmux)
        .await
        .expect("discover_sessions should succeed");
    let imported = fixture
        .client
        .import_session(discovered.sessions[0].route.clone(), discovered.sessions[0].title.clone())
        .await
        .expect("import_session should succeed");
    let topology = fixture
        .client
        .topology_snapshot(imported.session.session_id)
        .await
        .expect("topology_snapshot should succeed");
    let pane_id = topology.tabs[0].focused_pane.expect("focused pane should exist");
    let mut subscription = fixture
        .client
        .open_subscription(imported.session.session_id, SubscriptionSpec::PaneSurface { pane_id })
        .await
        .expect("subscription should open");

    let initial = must_recv_subscription_event(&mut subscription).await;
    let initial = match initial {
        SubscriptionEvent::ScreenDelta(delta) => delta,
        other => panic!("unexpected initial event: {other:?}"),
    };

    let dispatch = fixture
        .client
        .dispatch(
            imported.session.session_id,
            MuxCommand::SendInput(SendInputSpec {
                pane_id,
                data: submitted_input("hello from tmux subscription"),
                client_event_id: None,
            }),
        )
        .await
        .expect("send input should succeed");

    let updated = loop {
        let next = must_recv_subscription_event(&mut subscription).await;
        let next = match next {
            SubscriptionEvent::ScreenDelta(delta) => delta,
            other => panic!("unexpected pane event: {other:?}"),
        };
        let Some(patch) = next.patch.as_ref() else {
            continue;
        };
        if patch
            .line_updates
            .iter()
            .any(|line| line.line.text.contains("hello from tmux subscription"))
        {
            break next;
        }
    };
    let patch = updated.patch.expect("delta patch should exist");

    assert!(dispatch.changed);
    assert!(initial.full_replace.is_some());
    assert_ne!(updated.to_sequence, updated.from_sequence);
    assert!(
        patch
            .line_updates
            .iter()
            .any(|line| line.line.text.contains("hello from tmux subscription"))
    );
    assert!(updated.full_replace.is_none());

    fixture.shutdown().await.expect("fixture should stop cleanly");
}
