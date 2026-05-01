use std::sync::{Arc, Mutex as StdMutex};

use tokio::sync::Mutex;

use crate::{
    ZellijAttachedSession, ZellijBackend, ZellijPaneRow, ZellijTabRow, ZellijTarget,
    build_session_snapshot, collect_pane_ids,
};

pub(super) fn sample_attached_session() -> (
    ZellijAttachedSession,
    crate::ZellijSessionSnapshot,
    terminal_domain::TabId,
    terminal_domain::TabId,
    terminal_domain::PaneId,
    terminal_domain::PaneId,
) {
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
    let first_tab = snapshot.topology.tabs[0].tab_id;
    let second_tab = snapshot.topology.tabs[1].tab_id;
    let tab_one_panes = collect_pane_ids(&snapshot.topology.tabs[0].root);
    let attached = ZellijAttachedSession {
        backend: Arc::new(ZellijBackend),
        session_id,
        target,
        io_lane: Arc::new(StdMutex::new(())),
        command_lane: Arc::new(Mutex::new(())),
    };

    (attached, snapshot, first_tab, second_tab, tab_one_panes[0], tab_one_panes[1])
}

pub(super) fn single_tab_attached_session() -> (
    ZellijAttachedSession,
    crate::ZellijSessionSnapshot,
    terminal_domain::TabId,
    terminal_domain::PaneId,
) {
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
    let tab_id = snapshot.topology.tabs[0].tab_id;
    let pane_id = collect_pane_ids(&snapshot.topology.tabs[0].root)[0];
    let attached = ZellijAttachedSession {
        backend: Arc::new(ZellijBackend),
        session_id,
        target,
        io_lane: Arc::new(StdMutex::new(())),
        command_lane: Arc::new(Mutex::new(())),
    };

    (attached, snapshot, tab_id, pane_id)
}
