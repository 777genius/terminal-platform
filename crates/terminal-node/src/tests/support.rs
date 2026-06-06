use super::prelude::*;

pub(super) fn cat_launch_request(title: &str) -> NodeCreateSessionRequest {
    NodeCreateSessionRequest {
        title: Some(title.to_string()),
        launch: node_host_launch_spec().map(|launch| NodeShellLaunchSpec {
            program: launch.program,
            args: launch.args,
            cwd: launch.cwd.map(|cwd| cwd.display().to_string()),
        }),
    }
}

pub(super) fn node_host_launch_spec() -> Option<terminal_backend_api::ShellLaunchSpec> {
    #[cfg(unix)]
    {
        Some(echo_shell_launch_spec())
    }

    #[cfg(windows)]
    {
        None
    }
}

pub(super) async fn wait_for_screen_line(
    node: &NodeHostClient,
    session_id: &str,
    pane_id: &str,
    needle: &str,
) -> NodeScreenSnapshot {
    let mut last_lines = Vec::new();
    for _ in 0..screen_wait_attempts() {
        let snapshot = node
            .screen_snapshot(session_id, pane_id)
            .await
            .expect("screen_snapshot should succeed");
        if snapshot.surface.lines.iter().any(|line| line.text.contains(needle)) {
            return snapshot;
        }
        last_lines = snapshot.surface.lines.iter().map(|line| line.text.clone()).take(12).collect();
        sleep(Duration::from_millis(100)).await;
    }

    panic!("screen never contained expected line: {needle}; last lines: {last_lines:?}");
}

pub(super) async fn wait_for_interactive_screen(
    node: &NodeHostClient,
    session_id: &str,
    pane_id: &str,
    label: &str,
) -> NodeScreenSnapshot {
    let marker = format!("node-interactive-probe-{label}-{}", std::process::id());
    let mut last_lines = Vec::new();

    for attempt in 0..screen_wait_attempts() {
        if attempt % interactive_probe_interval() == 0 {
            timeout(
                operation_timeout(),
                node.dispatch_mux_command(
                    session_id,
                    &NodeMuxCommand::SendInput(NodeSendInputCommand {
                        pane_id: pane_id.to_string(),
                        data: submitted_input(&marker),
                        client_event_id: None,
                    }),
                ),
            )
            .await
            .expect("interactive probe send_input should not hang")
            .expect("interactive probe send_input should succeed");
        }

        let snapshot = node
            .screen_snapshot(session_id, pane_id)
            .await
            .expect("screen_snapshot should succeed");
        if snapshot.surface.lines.iter().any(|line| line.text.contains(&marker)) {
            return snapshot;
        }
        last_lines = snapshot.surface.lines.iter().map(|line| line.text.clone()).take(12).collect();
        sleep(Duration::from_millis(100)).await;
    }

    panic!("screen never reached interactive probe marker: {marker}; last lines: {last_lines:?}");
}

pub(super) async fn next_topology_snapshot(
    subscription: &NodeSubscriptionHandle,
) -> Option<NodeTopologySnapshot> {
    for _ in 0..20 {
        match timeout(subscription_timeout(), subscription.next_event())
            .await
            .expect("subscription next_event should not hang")
            .expect("subscription should stay healthy")
        {
            Some(NodeSubscriptionEvent::TopologySnapshot(snapshot)) => return Some(snapshot),
            Some(NodeSubscriptionEvent::ScreenDelta(_)) => continue,
            Some(NodeSubscriptionEvent::SessionHealthSnapshot(_)) => continue,
            None => return None,
        }
    }

    None
}

pub(super) async fn wait_for_topology_state(
    node: &NodeHostClient,
    session_id: &str,
    predicate: impl Fn(&NodeTopologySnapshot) -> bool,
    label: &str,
) -> NodeTopologySnapshot {
    for _ in 0..zellij_topology_wait_attempts() {
        let snapshot = timeout(extended_timeout(), node.topology_snapshot(session_id))
            .await
            .expect("topology_snapshot should not hang")
            .expect("topology_snapshot should succeed");
        if predicate(&snapshot) {
            return snapshot;
        }
        sleep(Duration::from_millis(100)).await;
    }

    panic!("topology never reached expected state: {label}");
}

pub(super) async fn wait_for_subscription_line(
    subscription: &NodeSubscriptionHandle,
    needle: &str,
) -> Option<NodeScreenDelta> {
    for _ in 0..screen_wait_attempts() {
        match timeout(subscription_timeout(), subscription.next_event())
            .await
            .expect("subscription next_event should not hang")
            .expect("subscription should stay healthy")
        {
            Some(NodeSubscriptionEvent::ScreenDelta(delta))
                if subscription_delta_contains(&delta, needle) =>
            {
                return Some(delta);
            }
            Some(NodeSubscriptionEvent::ScreenDelta(_)) => continue,
            Some(NodeSubscriptionEvent::TopologySnapshot(_)) => continue,
            Some(NodeSubscriptionEvent::SessionHealthSnapshot(_)) => continue,
            None => return None,
        }
    }

    None
}

pub(super) async fn wait_for_subscription_close(subscription: &NodeSubscriptionHandle) -> bool {
    timeout(subscription_timeout(), async {
        for _ in 0..screen_wait_attempts() {
            match subscription
                .next_event()
                .await
                .expect("subscription should stay healthy until closure")
            {
                Some(_) => continue,
                None => return true,
            }
        }

        false
    })
    .await
    .unwrap_or(false)
}

pub(super) fn subscription_timeout() -> Duration {
    if cfg!(windows) { Duration::from_secs(60) } else { Duration::from_secs(5) }
}

pub(super) fn operation_timeout() -> Duration {
    if cfg!(windows) { Duration::from_secs(60) } else { Duration::from_secs(5) }
}

pub(super) fn extended_timeout() -> Duration {
    if cfg!(windows) { Duration::from_secs(45) } else { Duration::from_secs(10) }
}

pub(super) fn zellij_operation_timeout() -> Duration {
    if cfg!(windows) { Duration::from_secs(60) } else { Duration::from_secs(90) }
}

pub(super) fn zellij_attempt_timeout() -> Duration {
    if cfg!(windows) { Duration::from_secs(120) } else { Duration::from_secs(90) }
}

pub(super) fn screen_wait_attempts() -> usize {
    if cfg!(windows) { 450 } else { 50 }
}

pub(super) fn interactive_probe_interval() -> usize {
    if cfg!(windows) { 20 } else { 10 }
}

pub(super) fn zellij_topology_wait_attempts() -> usize {
    if cfg!(windows) { 80 } else { 120 }
}

pub(super) fn submitted_input(text: &str) -> String {
    if cfg!(windows) { format!("{text}\r\n") } else { format!("{text}\n") }
}

pub(super) fn spawn_daemon_with_retry(
    address: terminal_protocol::LocalSocketAddress,
) -> std::io::Result<terminal_daemon::LocalSocketServerHandle> {
    let attempts = if cfg!(windows) { 50 } else { 5 };
    let retryable_kinds = [
        std::io::ErrorKind::AlreadyExists,
        std::io::ErrorKind::PermissionDenied,
        std::io::ErrorKind::AddrInUse,
    ];
    let mut last_error = None;

    for attempt in 0..attempts {
        match spawn_local_socket_server(daemon(), address.clone()) {
            Ok(server) => return Ok(server),
            Err(error) if retryable_kinds.contains(&error.kind()) && attempt + 1 < attempts => {
                last_error = Some(error);
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(error) => return Err(error),
        }
    }

    Err(last_error.unwrap_or_else(|| std::io::Error::other("daemon never rebound on address")))
}

pub(super) fn first_node_pane_id(root: &NodePaneTreeNode) -> Option<String> {
    match root {
        NodePaneTreeNode::Leaf { pane_id } => Some(pane_id.clone()),
        NodePaneTreeNode::Split(split) => {
            first_node_pane_id(&split.first).or_else(|| first_node_pane_id(&split.second))
        }
    }
}

pub(super) fn subscription_delta_contains(delta: &NodeScreenDelta, needle: &str) -> bool {
    delta
        .patch
        .as_ref()
        .map(|patch| patch.line_updates.iter().any(|line| line.line.text.contains(needle)))
        .unwrap_or(false)
        || delta
            .full_replace
            .as_ref()
            .map(|surface| surface.lines.iter().any(|line| line.text.contains(needle)))
            .unwrap_or(false)
}

pub(super) fn assert_zellij_delta_compatible_with_snapshot(
    snapshot: &NodeScreenSnapshot,
    delta: &NodeScreenDelta,
) {
    assert_eq!(delta.from_sequence, snapshot.sequence);
    assert!(
        delta.to_sequence >= snapshot.sequence,
        "zellij delta must not rewind sequence numbers"
    );
    if delta.to_sequence == snapshot.sequence {
        assert!(delta.patch.is_none());
        assert!(delta.full_replace.is_none());
    } else {
        assert!(delta.patch.is_none());
        assert!(delta.full_replace.is_some());
    }
}

pub(super) async fn wait_for_discovered_zellij_session(
    node: &NodeHostClient,
    session_name: &str,
) -> NodeDiscoveredSession {
    let started = Instant::now();
    while started.elapsed() < zellij_discovery_timeout() {
        let discovered = match timeout(
            extended_timeout(),
            node.discover_sessions(NodeBackendKind::Zellij),
        )
        .await
        {
            Ok(Ok(discovered)) => discovered,
            Ok(Err(_)) | Err(_) => break,
        };
        if let Some(candidate) =
            discovered.into_iter().find(|session| session.title.as_deref() == Some(session_name))
        {
            return candidate;
        }
        sleep(Duration::from_millis(100)).await;
    }

    fallback_zellij_candidate(session_name)
}

pub(super) fn zellij_discovery_timeout() -> Duration {
    if cfg!(windows) { Duration::from_secs(30) } else { Duration::from_secs(20) }
}

pub(super) fn fallback_zellij_candidate(session_name: &str) -> NodeDiscoveredSession {
    NodeDiscoveredSession {
        route: NodeSessionRoute {
            backend: NodeBackendKind::Zellij,
            authority: NodeRouteAuthority::ImportedForeign,
            external: Some(NodeExternalSessionRef {
                namespace: "zellij_session".to_string(),
                value: format!("session={session_name}"),
            }),
        },
        title: Some(session_name.to_string()),
    }
}
