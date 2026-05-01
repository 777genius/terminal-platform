use super::prelude::*;

#[cfg(any(unix, windows))]
pub(super) fn cat_launch_spec() -> ShellLaunchSpec {
    echo_shell_launch_spec()
}

#[cfg(unix)]
pub(super) fn daemon_with_incompatible_saved_session(
    label: &str,
    manifest: SavedSessionManifest,
) -> (TerminalDaemon, SessionId) {
    let store = SqliteSessionStore::open(unique_sqlite_path(label))
        .expect("isolated sqlite session store should open");
    let session_id = SessionId::new();
    let tab_id = TabId::new();
    let pane_id = PaneId::new();
    store
        .save_native_session(&terminal_persistence::SavedNativeSession {
            session_id,
            route: local_native_route(session_id),
            title: Some("future-shell".to_string()),
            launch: None,
            manifest,
            topology: TopologySnapshot {
                session_id,
                backend_kind: BackendKind::Native,
                tabs: vec![TabSnapshot {
                    tab_id,
                    title: Some("future-shell".to_string()),
                    root: PaneTreeNode::Leaf { pane_id },
                    focused_pane: Some(pane_id),
                }],
                focused_tab: Some(tab_id),
            },
            screens: Vec::new(),
            saved_at_ms: SqliteSessionStore::save_timestamp_ms()
                .expect("save timestamp should resolve"),
        })
        .expect("future snapshot should save");

    (TerminalDaemon::with_persistence(store), session_id)
}

#[cfg(any(unix, windows))]
pub(super) async fn wait_for_screen_line(
    fixture: &terminal_testing::DaemonFixture,
    session_id: terminal_domain::SessionId,
    pane_id: terminal_domain::PaneId,
    needle: &str,
) {
    let mut last_lines = Vec::new();
    for _ in 0..120 {
        let screen = fixture
            .client
            .screen_snapshot(session_id, pane_id)
            .await
            .expect("screen_snapshot should succeed");
        if screen.surface.lines.iter().any(|line| line.text.contains(needle)) {
            return;
        }
        last_lines = screen.surface.lines.iter().map(|line| line.text.clone()).take(12).collect();
        sleep(Duration::from_millis(50)).await;
    }

    panic!("screen never contained expected text: {needle}; last lines: {last_lines:?}");
}

#[cfg(any(unix, windows))]
pub(super) async fn wait_for_topology(
    fixture: &terminal_testing::DaemonFixture,
    session_id: terminal_domain::SessionId,
    predicate: impl Fn(&TopologySnapshot) -> bool,
    label: &str,
) -> TopologySnapshot {
    let attempts = if label.contains("zellij") { zellij_topology_wait_attempts() } else { 120 };
    let mut last_snapshot = None;
    for _ in 0..attempts {
        let snapshot =
            tokio::time::timeout(host_timeout(), fixture.client.topology_snapshot(session_id))
                .await
                .expect("topology_snapshot should not hang")
                .expect("topology_snapshot should succeed");
        if predicate(&snapshot) {
            return snapshot;
        }
        last_snapshot = Some(snapshot);
        sleep(Duration::from_millis(50)).await;
    }

    panic!("topology never reached expected state: {label}; last snapshot: {last_snapshot:?}");
}

#[cfg(any(unix, windows))]
pub(super) async fn recv_subscription_event(
    subscription: &mut terminal_daemon_client::LocalSocketSubscription,
) -> Option<SubscriptionEvent> {
    tokio::time::timeout(host_timeout(), subscription.recv())
        .await
        .expect("subscription recv should not hang")
        .expect("subscription recv should succeed")
}

#[cfg(any(unix, windows))]
pub(super) fn zellij_operation_timeout() -> Duration {
    if cfg!(windows) { Duration::from_secs(60) } else { Duration::from_secs(90) }
}

#[cfg(any(unix, windows))]
pub(super) fn zellij_attempt_timeout() -> Duration {
    if cfg!(windows) { Duration::from_secs(120) } else { Duration::from_secs(90) }
}

#[cfg(any(unix, windows))]
pub(super) fn host_timeout() -> Duration {
    if cfg!(windows) { Duration::from_secs(45) } else { Duration::from_secs(10) }
}

#[cfg(any(unix, windows))]
pub(super) fn zellij_topology_wait_attempts() -> usize {
    if cfg!(windows) { 80 } else { 120 }
}

#[cfg(any(unix, windows))]
pub(super) fn assert_zellij_delta_compatible_with_snapshot(
    snapshot: &ScreenSnapshot,
    delta: &ScreenDelta,
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

#[cfg(any(unix, windows))]
pub(super) async fn wait_for_discovered_zellij_session(
    client: &terminal_daemon_client::LocalSocketDaemonClient,
    session_name: &str,
) -> terminal_backend_api::DiscoveredSession {
    let started = Instant::now();
    while started.elapsed() < zellij_discovery_timeout() {
        let discovered = match tokio::time::timeout(
            host_timeout(),
            client.discover_sessions(BackendKind::Zellij),
        )
        .await
        {
            Ok(Ok(discovered)) => discovered,
            Ok(Err(_)) | Err(_) => break,
        };
        if let Some(candidate) = discovered
            .sessions
            .into_iter()
            .find(|session| session.title.as_deref() == Some(session_name))
        {
            return candidate;
        }
        sleep(Duration::from_millis(100)).await;
    }

    fallback_zellij_candidate(session_name)
}

#[cfg(any(unix, windows))]
pub(super) fn zellij_discovery_timeout() -> Duration {
    if cfg!(windows) { Duration::from_secs(30) } else { Duration::from_secs(20) }
}

#[cfg(any(unix, windows))]
pub(super) fn fallback_zellij_candidate(
    session_name: &str,
) -> terminal_backend_api::DiscoveredSession {
    terminal_backend_api::DiscoveredSession {
        route: terminal_domain::SessionRoute {
            backend: BackendKind::Zellij,
            authority: terminal_domain::RouteAuthority::ImportedForeign,
            external: Some(terminal_domain::ExternalSessionRef {
                namespace: "zellij_session".to_string(),
                value: format!("session={session_name}"),
            }),
        },
        title: Some(session_name.to_string()),
    }
}

#[cfg(any(unix, windows))]
pub(super) fn submitted_input(text: &str) -> String {
    if cfg!(windows) { format!("{text}\r\n") } else { format!("{text}\n") }
}

#[cfg(any(unix, windows))]
pub(super) async fn must_recv_subscription_event(
    subscription: &mut terminal_daemon_client::LocalSocketSubscription,
) -> SubscriptionEvent {
    recv_subscription_event(subscription).await.expect("subscription should emit an event")
}

#[cfg(any(unix, windows))]
pub(super) fn collect_pane_ids(root: &PaneTreeNode) -> Vec<PaneId> {
    let mut pane_ids = Vec::new();
    collect_pane_ids_inner(root, &mut pane_ids);
    pane_ids
}

#[cfg(any(unix, windows))]
pub(super) fn collect_pane_ids_inner(root: &PaneTreeNode, pane_ids: &mut Vec<PaneId>) {
    match root {
        PaneTreeNode::Leaf { pane_id } => pane_ids.push(*pane_id),
        PaneTreeNode::Split(split) => {
            collect_pane_ids_inner(&split.first, pane_ids);
            collect_pane_ids_inner(&split.second, pane_ids);
        }
    }
}

#[cfg(unix)]
pub(super) fn tmux_daemon(socket_name: &str) -> TerminalDaemon {
    TerminalDaemon::new(TerminalRuntime::new(BackendCatalog::new([
        Arc::new(NativeBackend::default()) as Arc<dyn MuxBackendPort>,
        Arc::new(TmuxBackend::with_socket_name(socket_name)) as Arc<dyn MuxBackendPort>,
        Arc::new(ZellijBackend) as Arc<dyn MuxBackendPort>,
    ])))
}

#[cfg(unix)]
pub(super) fn unique_tmux_socket_name(label: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("terminal-platform-{label}-{}-{nanos}", std::process::id())
}

#[cfg(unix)]
pub(super) fn unique_tmux_session_name(label: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("{label}-{}-{nanos}", std::process::id())
}

#[cfg(unix)]
pub(super) struct TmuxServerGuard {
    socket_name: String,
}

#[cfg(unix)]
impl TmuxServerGuard {
    pub(super) fn spawn(socket_name: &str, session_name: &str) -> Result<Self, String> {
        Self::spawn_with_commands(
            socket_name,
            session_name,
            "printf 'hello from tmux\\n'; exec cat",
            "printf 'logs ready\\n'; exec cat",
        )
    }

    pub(super) fn spawn_with_shell(socket_name: &str, session_name: &str) -> Result<Self, String> {
        Self::spawn_with_commands(
            socket_name,
            session_name,
            "printf 'hello from tmux\\n'; exec env PS1='terminal-platform$ ' sh -i",
            "printf 'logs ready\\n'; exec env PS1='terminal-platform$ ' sh -i",
        )
    }

    pub(super) fn spawn_with_commands(
        socket_name: &str,
        session_name: &str,
        main_command: &str,
        secondary_command: &str,
    ) -> Result<Self, String> {
        run_tmux(
            socket_name,
            &["new-session", "-d", "-s", session_name, "sh", "-lc", main_command],
        )?;
        run_tmux(
            socket_name,
            &["new-window", "-d", "-t", session_name, "-n", "logs", "sh", "-lc", secondary_command],
        )?;

        Ok(Self { socket_name: socket_name.to_string() })
    }
}

#[cfg(unix)]
impl Drop for TmuxServerGuard {
    pub(super) fn drop(&mut self) {
        let _ = run_tmux(&self.socket_name, &["kill-server"]);
    }
}

#[cfg(unix)]
pub(super) fn run_tmux(socket_name: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new("tmux")
        .arg("-L")
        .arg(socket_name)
        .args(args)
        .output()
        .map_err(|error| format!("failed to spawn tmux: {error}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }

    String::from_utf8(output.stdout).map_err(|error| format!("invalid tmux utf8 output: {error}"))
}
