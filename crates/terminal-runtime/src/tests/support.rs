use super::{fakes::*, prelude::*};

pub(super) fn runtime_backends(imported_backend: Arc<FakeImportedBackend>) -> BackendCatalog {
    BackendCatalog::new([
        Arc::new(NativeBackend::default()) as Arc<dyn MuxBackendPort>,
        imported_backend as Arc<dyn MuxBackendPort>,
    ])
}

pub(super) fn foreign_route(value: &str) -> SessionRoute {
    SessionRoute {
        backend: BackendKind::Tmux,
        authority: RouteAuthority::ImportedForeign,
        external: Some(ExternalSessionRef {
            namespace: "tmux_session".to_string(),
            value: value.to_string(),
        }),
    }
}

pub(super) fn unique_runtime_store_path(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    std::env::temp_dir()
        .join(format!("terminal-runtime-{label}-{}-{nanos}.sqlite3", std::process::id()))
}

pub(super) fn capture_shell_launch_spec() -> terminal_backend_api::ShellLaunchSpec {
    #[cfg(windows)]
    {
        let program = std::env::var("COMSPEC")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "cmd.exe".to_string());

        terminal_backend_api::ShellLaunchSpec::new(program).with_args([
            "/D",
            "/Q",
            "/K",
            "echo ready",
        ])
    }

    #[cfg(not(windows))]
    {
        terminal_backend_api::ShellLaunchSpec::new("/bin/sh")
            .with_args(["-lc", "printf 'ready\\n'; exec cat"])
    }
}

pub(super) fn capture_shell_echo_input(text: &str) -> String {
    #[cfg(windows)]
    {
        format!("echo {text}\r")
    }

    #[cfg(not(windows))]
    {
        format!("{text}\r")
    }
}

pub(super) async fn wait_for_runtime_screen_line(
    runtime: &TerminalRuntime,
    session_id: SessionId,
    pane_id: PaneId,
    needle: &str,
) -> Option<()> {
    for _ in 0..120 {
        if let Ok(screen) = runtime.screen_snapshot(session_id, pane_id).await
            && screen.surface.lines.iter().any(|line| line.text.contains(needle))
        {
            return Some(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    None
}

pub(super) async fn wait_for_runtime_topology<F>(
    runtime: &TerminalRuntime,
    session_id: SessionId,
    predicate: F,
) -> Option<TopologySnapshot>
where
    F: Fn(&TopologySnapshot) -> bool,
{
    for _ in 0..120 {
        if let Ok(topology) = runtime.topology_snapshot(session_id).await
            && predicate(&topology)
        {
            return Some(topology);
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    None
}

pub(super) fn collect_test_pane_ids(root: &PaneTreeNode) -> Vec<PaneId> {
    let mut pane_ids = Vec::new();
    collect_test_pane_ids_inner(root, &mut pane_ids);
    pane_ids
}

pub(super) fn collect_test_pane_ids_inner(root: &PaneTreeNode, pane_ids: &mut Vec<PaneId>) {
    match root {
        PaneTreeNode::Leaf { pane_id } => pane_ids.push(*pane_id),
        PaneTreeNode::Split(split) => {
            collect_test_pane_ids_inner(&split.first, pane_ids);
            collect_test_pane_ids_inner(&split.second, pane_ids);
        }
    }
}

pub(super) async fn wait_for_v2_snapshot(
    path: &Path,
    session_id: SessionId,
) -> Option<terminal_persistence::RestorePlan> {
    for _ in 0..80 {
        if let Ok(store) = terminal_persistence::TerminalPersistenceV2::open_with_config(
            path,
            terminal_persistence::TerminalPersistenceV2Config::test(),
        ) && let Ok(plan) = store.restore_plan(&session_id.0.to_string())
            && plan.latest_screen_snapshot_id.is_some()
        {
            return Some(plan);
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    None
}

pub(super) async fn wait_for_v2_restore_plan<F>(
    path: &Path,
    session_id: SessionId,
    predicate: F,
) -> Option<terminal_persistence::RestorePlan>
where
    F: Fn(&terminal_persistence::RestorePlan) -> bool,
{
    for _ in 0..120 {
        if let Ok(store) = terminal_persistence::TerminalPersistenceV2::open_with_config(
            path,
            terminal_persistence::TerminalPersistenceV2Config::test(),
        ) && let Ok(plan) = store.restore_plan(&session_id.0.to_string())
            && predicate(&plan)
        {
            return Some(plan);
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    None
}

pub(super) async fn wait_for_v2_payload(
    path: &Path,
    session_id: SessionId,
    pane_id: PaneId,
    needle: &[u8],
) -> Option<Vec<u8>> {
    for _ in 0..120 {
        if let Ok(store) = terminal_persistence::TerminalPersistenceV2::open_with_config(
            path,
            terminal_persistence::TerminalPersistenceV2Config::test(),
        ) && let Ok(segments) =
            store.list_stream_segments(&session_id.0.to_string(), &pane_id.0.to_string(), 1, 512)
        {
            let payload =
                segments.into_iter().flat_map(|segment| segment.payload).collect::<Vec<_>>();
            if payload.windows(needle.len()).any(|window| window == needle) {
                return Some(payload);
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    None
}

pub(super) async fn wait_for_v2_command_history(
    path: &Path,
    session_id: SessionId,
) -> Option<Vec<terminal_persistence::CommandHistoryEntryRecord>> {
    for _ in 0..80 {
        if let Ok(store) = terminal_persistence::TerminalPersistenceV2::open_with_config(
            path,
            terminal_persistence::TerminalPersistenceV2Config::test(),
        ) && let Ok(history) = store.list_command_history(Some(&session_id.0.to_string()), 10)
            && !history.is_empty()
        {
            return Some(history);
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    None
}

pub(super) fn seed_legacy_saved_session_schema(path: &Path) {
    let connection = Connection::open(path).expect("legacy sqlite file should open");
    connection
        .execute_batch(
            "
            CREATE TABLE native_saved_sessions (
                session_id TEXT PRIMARY KEY,
                route_json TEXT NOT NULL,
                title TEXT,
                launch_json TEXT,
                manifest_json TEXT NOT NULL,
                topology_json TEXT NOT NULL,
                screens_json TEXT NOT NULL,
                saved_at_ms INTEGER NOT NULL
            );
            CREATE TABLE __rusqlite_migration_schema_history (
                version INTEGER PRIMARY KEY,
                description TEXT NOT NULL,
                installed_on TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                success INTEGER NOT NULL
            );
            INSERT INTO __rusqlite_migration_schema_history (version, description, success)
            VALUES (0, 'initial native_saved_sessions schema', 1), (1, 'persistence bootstrap noop', 1);
            ",
        )
        .expect("legacy schema should seed");
}
