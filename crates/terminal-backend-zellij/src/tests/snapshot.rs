use crate::{
    ZellijPaneKind, ZellijPaneRow, ZellijTabRow, ZellijTarget, build_session_snapshot,
    collect_pane_ids, dump_screen_scrollback_args,
};

#[test]
fn builds_session_snapshot_from_rich_cli_rows() {
    let target = ZellijTarget { session_name: "workspace".to_string() };
    let session_id = terminal_domain::SessionId::new();
    let tabs = vec![
        ZellijTabRow { tab_id: 1, position: 0, name: "shell".to_string(), active: true },
        ZellijTabRow { tab_id: 2, position: 1, name: "logs".to_string(), active: false },
    ];
    let panes = vec![
        ZellijPaneRow {
            id: 1,
            tab_id: 1,
            title: "shell".to_string(),
            is_plugin: false,
            is_focused: true,
            is_floating: false,
            pane_x: 0,
            pane_y: 0,
            pane_rows: 24,
            pane_columns: 80,
        },
        ZellijPaneRow {
            id: 2,
            tab_id: 1,
            title: "status".to_string(),
            is_plugin: true,
            is_focused: false,
            is_floating: false,
            pane_x: 81,
            pane_y: 0,
            pane_rows: 24,
            pane_columns: 40,
        },
        ZellijPaneRow {
            id: 3,
            tab_id: 2,
            title: "logs".to_string(),
            is_plugin: false,
            is_focused: false,
            is_floating: false,
            pane_x: 0,
            pane_y: 0,
            pane_rows: 24,
            pane_columns: 100,
        },
    ];

    let snapshot =
        build_session_snapshot(session_id, &target, &tabs, &panes).expect("snapshot should build");

    assert_eq!(snapshot.topology.backend_kind, terminal_domain::BackendKind::Zellij);
    assert_eq!(snapshot.topology.tabs.len(), 2);
    assert_eq!(snapshot.topology.focused_tab, Some(snapshot.topology.tabs[0].tab_id));
    assert_eq!(
        snapshot.topology.tabs[0].focused_pane,
        Some(collect_pane_ids(&snapshot.topology.tabs[0].root)[0])
    );
    assert_eq!(snapshot.tab_targets.len(), 2);
    assert_eq!(snapshot.pane_targets.len(), 3);
    assert!(snapshot.pane_targets.values().any(|pane| pane.backend_ref == "plugin_2"));
    assert!(snapshot.pane_targets.values().any(|pane| pane.kind == ZellijPaneKind::Plugin));
}

#[test]
fn zellij_screen_snapshot_requests_full_scrollback_for_pane() {
    let target = ZellijTarget { session_name: "workspace".to_string() };
    let session_id = terminal_domain::SessionId::new();
    let tabs =
        vec![ZellijTabRow { tab_id: 1, position: 0, name: "shell".to_string(), active: true }];
    let panes = vec![ZellijPaneRow {
        id: 1,
        tab_id: 1,
        title: "shell".to_string(),
        is_plugin: false,
        is_focused: true,
        is_floating: false,
        pane_x: 0,
        pane_y: 0,
        pane_rows: 24,
        pane_columns: 80,
    }];
    let snapshot =
        build_session_snapshot(session_id, &target, &tabs, &panes).expect("snapshot should build");
    let pane_target = snapshot.pane_targets.values().next().expect("pane target should exist");

    assert_eq!(
        dump_screen_scrollback_args(pane_target),
        vec!["action", "dump-screen", "--pane-id", "terminal_1", "--full"]
    );
}
