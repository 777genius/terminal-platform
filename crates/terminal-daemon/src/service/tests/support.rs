use super::prelude::*;

#[cfg(feature = "native-backend")]
pub(super) fn isolated_store_path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "terminal-platform-daemon-service-{label}-{}-{}.sqlite3",
        std::process::id(),
        terminal_domain::SessionId::new().0
    ))
}

#[cfg(feature = "native-backend")]
pub(super) fn cat_launch_spec() -> ShellLaunchSpec {
    #[cfg(unix)]
    {
        ShellLaunchSpec::new("/bin/sh").with_args(["-lc", "printf 'ready\\n'; exec cat"])
    }

    #[cfg(windows)]
    {
        let program = std::env::var("COMSPEC")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "cmd.exe".to_string());

        ShellLaunchSpec::new(program).with_args(["/D", "/Q", "/K", "echo ready"])
    }
}

#[cfg(feature = "native-backend")]
pub(super) fn isolated_daemon() -> TerminalDaemon {
    let store = SqliteSessionStore::open(isolated_store_path("test"))
        .expect("isolated sqlite session store should open");

    TerminalDaemon::with_persistence(store)
}

#[cfg(feature = "native-backend")]
pub(super) fn save_incompatible_snapshot(
    label: &str,
    manifest: SavedSessionManifest,
) -> (TerminalDaemon, terminal_domain::SessionId) {
    let path = isolated_store_path(label);
    let store = SqliteSessionStore::open(&path).expect("isolated sqlite session store should open");
    let session_id = terminal_domain::SessionId::new();
    let tab_id = terminal_domain::TabId::new();
    let pane_id = terminal_domain::PaneId::new();
    store
        .save_native_session(&terminal_persistence::SavedNativeSession {
            session_id,
            route: local_native_route(session_id),
            title: Some("future-shell".to_string()),
            launch: None,
            manifest,
            topology: TopologySnapshot {
                session_id,
                backend_kind: terminal_domain::BackendKind::Native,
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
