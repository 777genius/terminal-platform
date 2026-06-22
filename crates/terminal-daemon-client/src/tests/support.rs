use super::prelude::*;

pub(super) fn unique_address(label: &str) -> terminal_protocol::LocalSocketAddress {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let slug = format!("terminal-platform-{label}-{}-{nanos}.sock", std::process::id());

    terminal_protocol::LocalSocketAddress::from_runtime_slug(slug)
}

pub(super) fn spawn_default_daemon_with_retry(
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
        match spawn_local_socket_server(isolated_daemon(), address.clone()) {
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

pub(super) fn isolated_daemon() -> TerminalDaemon {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let store =
        SqliteSessionStore::open(std::env::temp_dir().join(format!(
            "terminal-platform-daemon-client-{}-{nanos}.sqlite3",
            std::process::id()
        )))
        .expect("isolated sqlite session store should open");

    TerminalDaemon::with_persistence(store)
}

#[cfg(unix)]
pub(super) fn isolated_daemon_with_saved_snapshot(
    label: &str,
    manifest: SavedSessionManifest,
) -> (TerminalDaemon, SessionId) {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let path = std::env::temp_dir().join(format!(
        "terminal-platform-daemon-client-{label}-{}-{nanos}.sqlite3",
        std::process::id()
    ));
    let store = SqliteSessionStore::open(&path).expect("isolated sqlite session store should open");
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

#[cfg(unix)]
pub(super) fn isolated_daemon_with_valid_and_corrupted_saved_rows(
    label: &str,
) -> (TerminalDaemon, SessionId, SessionId) {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let path = std::env::temp_dir().join(format!(
        "terminal-platform-daemon-client-{label}-{}-{nanos}.sqlite3",
        std::process::id()
    ));
    let store = SqliteSessionStore::open(&path).expect("isolated sqlite session store should open");
    let valid_session_id = SessionId::new();
    let corrupt_session_id = SessionId::new();
    let tab_id = TabId::new();
    let pane_id = PaneId::new();
    let manifest = SavedSessionManifest::current();
    let route = local_native_route(valid_session_id);
    let launch = Some(cat_launch_spec());
    store
        .save_native_session(&terminal_persistence::SavedNativeSession {
            session_id: valid_session_id,
            route: route.clone(),
            title: Some("healthy-shell".to_string()),
            launch: launch.clone(),
            manifest: manifest.clone(),
            topology: TopologySnapshot {
                session_id: valid_session_id,
                backend_kind: BackendKind::Native,
                tabs: vec![TabSnapshot {
                    tab_id,
                    title: Some("healthy-shell".to_string()),
                    root: PaneTreeNode::Leaf { pane_id },
                    focused_pane: Some(pane_id),
                }],
                focused_tab: Some(tab_id),
            },
            screens: Vec::new(),
            saved_at_ms: SqliteSessionStore::save_timestamp_ms()
                .expect("save timestamp should resolve"),
        })
        .expect("valid snapshot should save");

    let connection = Connection::open(&path).expect("raw sqlite should open");
    connection
        .execute(
            "
            INSERT INTO native_saved_sessions (
                session_id,
                route_json,
                title,
                launch_json,
                manifest_json,
                topology_json,
                screens_json,
                saved_at_ms
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ",
            params![
                corrupt_session_id.0.to_string(),
                serde_json::to_string(&route).expect("route should serialize"),
                "corrupt-shell",
                serde_json::to_string(&launch).expect("launch should serialize"),
                serde_json::to_string(&manifest).expect("manifest should serialize"),
                "{not-json",
                serde_json::to_string::<Vec<terminal_projection::ScreenSnapshot>>(&Vec::new())
                    .expect("screens should serialize"),
                SqliteSessionStore::save_timestamp_ms().expect("save timestamp should resolve") + 1,
            ],
        )
        .expect("corrupted row should insert");

    (TerminalDaemon::with_persistence(store), valid_session_id, corrupt_session_id)
}

#[cfg(unix)]
pub(super) fn cat_launch_spec() -> ShellLaunchSpec {
    ShellLaunchSpec::new("/bin/sh").with_args(["-lc", "printf 'ready\\n'; exec cat"])
}

#[cfg(unix)]
pub(super) fn quiet_launch_spec() -> ShellLaunchSpec {
    ShellLaunchSpec::new("/bin/sh").with_args(["-c", "exec sleep 60"])
}

#[cfg(unix)]
pub(super) fn submitted_input(text: &str) -> String {
    if cfg!(windows) { format!("{text}\r\n") } else { format!("{text}\n") }
}

#[cfg(unix)]
pub(super) async fn wait_for_screen_line(
    client: &LocalSocketDaemonClient,
    session_id: terminal_domain::SessionId,
    pane_id: terminal_domain::PaneId,
    needle: &str,
) {
    let mut last_lines = Vec::new();
    for _ in 0..120 {
        let screen = client
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

pub(super) async fn recv_subscription_event(
    subscription: &mut LocalSocketSubscription,
) -> Option<SubscriptionEvent> {
    timeout(Duration::from_secs(5), subscription.recv())
        .await
        .expect("subscription recv should not hang")
        .expect("subscription recv should succeed")
}

pub(super) async fn must_recv_subscription_event(
    subscription: &mut LocalSocketSubscription,
) -> SubscriptionEvent {
    recv_subscription_event(subscription).await.expect("subscription should emit an event")
}
