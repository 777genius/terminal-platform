use std::sync::{Arc, Mutex as StdMutex};

use terminal_backend_api::{
    BackendErrorKind, MuxCommand, NewTabSpec, SendInputSpec, SendPasteSpec,
};
use terminal_domain::{DegradedModeReason, RouteAuthority, SessionRoute};
use tokio::sync::Mutex;

use super::{
    ZELLIJ_ROUTE_NAMESPACE, ZellijAction, ZellijAttachedSession, ZellijBackend, ZellijPaneKind,
    ZellijPaneRow, ZellijProbe, ZellijSurface, ZellijTabRow, ZellijTarget, build_session_snapshot,
    collect_pane_ids, parse_panes_json, parse_semver_triplet, parse_tabs_json,
};

#[test]
fn parses_legacy_surface_from_cli_help() {
    let probe = ZellijProbe::parse(
        "zellij 0.43.1",
        Some("SUBCOMMANDS:\n    action\n    attach\n"),
        Some("SUBCOMMANDS:\n    dump-layout\n    query-tab-names\n"),
    );

    assert_eq!(probe.surface, ZellijSurface::LegacyCli043);
}

#[test]
fn parses_rich_surface_from_cli_help() {
    let probe = ZellijProbe::parse(
        "zellij 0.44.1",
        Some("SUBCOMMANDS:\n    action\n    subscribe\n"),
        Some("SUBCOMMANDS:\n    list-panes\n    list-tabs\n"),
    );

    assert_eq!(probe.surface, ZellijSurface::RichCli044Plus);
}

#[test]
fn falls_back_to_version_when_help_is_missing() {
    let probe = ZellijProbe::parse("zellij 0.43.1", None, None);

    assert_eq!(probe.surface, ZellijSurface::LegacyCli043);
}

#[test]
fn parses_semver_triplet() {
    assert_eq!(parse_semver_triplet("0.43.1"), Some((0, 43, 1)));
    assert_eq!(parse_semver_triplet("v0.44.0"), Some((0, 44, 0)));
}

#[test]
fn roundtrips_zellij_route_target() {
    let route = SessionRoute {
        backend: terminal_domain::BackendKind::Zellij,
        authority: RouteAuthority::ImportedForeign,
        external: Some(terminal_domain::ExternalSessionRef {
            namespace: ZELLIJ_ROUTE_NAMESPACE.to_string(),
            value: "session=workspace".to_string(),
        }),
    };

    let target = ZellijTarget::from_route(&route).expect("route should decode");
    assert_eq!(target.session_name, "workspace");
}

#[test]
fn rejects_invalid_zellij_route_namespace() {
    let route = SessionRoute {
        backend: terminal_domain::BackendKind::Zellij,
        authority: RouteAuthority::ImportedForeign,
        external: Some(terminal_domain::ExternalSessionRef {
            namespace: "other".to_string(),
            value: "session=workspace".to_string(),
        }),
    };

    let error = ZellijTarget::from_route(&route).expect_err("route should fail");
    assert_eq!(error.kind, terminal_backend_api::BackendErrorKind::InvalidInput);
}

#[test]
fn parses_rich_tab_rows_from_json() {
    let tabs = parse_tabs_json(
        r#"
            [
              { "tab_id": 1, "position": 0, "name": "shell", "active": true },
              { "tab_id": 2, "position": 1, "name": "logs", "active": false }
            ]
            "#,
    )
    .expect("tab rows should decode");

    assert_eq!(
        tabs,
        vec![
            ZellijTabRow { tab_id: 1, position: 0, name: "shell".to_string(), active: true },
            ZellijTabRow { tab_id: 2, position: 1, name: "logs".to_string(), active: false },
        ]
    );
}

#[test]
fn parses_rich_pane_rows_from_json() {
    let panes = parse_panes_json(
        r#"
            [
              {
                "id": 1,
                "tab_id": 1,
                "title": "shell",
                "is_plugin": false,
                "is_focused": true,
                "is_floating": false,
                "pane_x": 0,
                "pane_y": 0,
                "pane_rows": 24,
                "pane_columns": 80
              },
              {
                "id": 2,
                "tab_id": 1,
                "title": "status",
                "is_plugin": true,
                "is_focused": false,
                "is_floating": false,
                "pane_x": 81,
                "pane_y": 0,
                "pane_rows": 24,
                "pane_columns": 40
              }
            ]
            "#,
    )
    .expect("pane rows should decode");

    assert_eq!(panes.len(), 2);
    assert_eq!(panes[0].backend_ref(), "terminal_1");
    assert_eq!(panes[1].backend_ref(), "plugin_2");
}

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
fn builds_targeted_dispatch_actions_for_rich_surface() {
    let (attached, snapshot, first_tab, second_tab, terminal_pane, _plugin_pane) =
        sample_attached_session();

    assert_eq!(
        attached
            .dispatch_actions(
                &snapshot,
                MuxCommand::NewTab(NewTabSpec { title: Some("debug".to_string()) }),
            )
            .expect("new-tab should map"),
        vec![ZellijAction::NewTab { title: Some("debug".to_string()) }]
    );
    if cfg!(windows) {
        let error = attached
            .dispatch_actions(&snapshot, MuxCommand::FocusTab { tab_id: second_tab })
            .expect_err("windows focus-tab should be unsupported");
        assert_eq!(error.kind, BackendErrorKind::Unsupported);
    } else {
        assert_eq!(
            attached
                .dispatch_actions(&snapshot, MuxCommand::FocusTab { tab_id: second_tab })
                .expect("focus-tab should map"),
            vec![ZellijAction::FocusTab { backend_tab_id: 2, display_index: 1 }]
        );
    }
    assert_eq!(
        attached
            .dispatch_actions(
                &snapshot,
                MuxCommand::RenameTab { tab_id: second_tab, title: "renamed".to_string() },
            )
            .expect("rename-tab should map"),
        vec![ZellijAction::RenameTab { backend_tab_id: 2, title: "renamed".to_string() }]
    );
    assert_eq!(
        attached
            .dispatch_actions(&snapshot, MuxCommand::CloseTab { tab_id: second_tab })
            .expect("close-tab should map"),
        vec![ZellijAction::CloseTab { backend_tab_id: 2 }]
    );
    if cfg!(windows) {
        let error = attached
            .dispatch_actions(&snapshot, MuxCommand::FocusPane { pane_id: terminal_pane })
            .expect_err("windows focus-pane should be unsupported");
        assert_eq!(error.kind, BackendErrorKind::Unsupported);
    } else {
        assert_eq!(
            attached
                .dispatch_actions(&snapshot, MuxCommand::FocusPane { pane_id: terminal_pane })
                .expect("focus-pane should map"),
            vec![ZellijAction::FocusPane { pane_ref: "terminal_1".to_string() }]
        );
    }
    assert_eq!(
        attached
            .dispatch_actions(&snapshot, MuxCommand::ClosePane { pane_id: terminal_pane })
            .expect("close-pane should map"),
        vec![ZellijAction::ClosePane { pane_ref: "terminal_1".to_string() }]
    );
    assert_ne!(first_tab, second_tab);
}

#[test]
fn splits_terminal_input_into_ordered_rich_actions() {
    let (attached, snapshot, _first_tab, _second_tab, terminal_pane, _plugin_pane) =
        sample_attached_session();

    let actions = attached
        .dispatch_actions(
            &snapshot,
            MuxCommand::SendInput(SendInputSpec {
                pane_id: terminal_pane,
                data: "echo\tok\r\u{1b}[A\u{0003}\u{007f}\r\n".to_string(),
                client_event_id: None,
            }),
        )
        .expect("send-input should map");

    assert_eq!(
        actions,
        vec![
            ZellijAction::WriteChars {
                pane_ref: "terminal_1".to_string(),
                chars: "echo".to_string(),
            },
            ZellijAction::SendKeys {
                pane_ref: "terminal_1".to_string(),
                keys: vec!["Tab".to_string()],
            },
            ZellijAction::WriteChars {
                pane_ref: "terminal_1".to_string(),
                chars: "ok".to_string(),
            },
            ZellijAction::SendKeys {
                pane_ref: "terminal_1".to_string(),
                keys: vec!["Enter".to_string()],
            },
            ZellijAction::SendKeys {
                pane_ref: "terminal_1".to_string(),
                keys: vec!["Up".to_string()],
            },
            ZellijAction::SendKeys {
                pane_ref: "terminal_1".to_string(),
                keys: vec!["Ctrl c".to_string()],
            },
            ZellijAction::SendKeys {
                pane_ref: "terminal_1".to_string(),
                keys: vec!["Backspace".to_string()],
            },
            ZellijAction::SendKeys {
                pane_ref: "terminal_1".to_string(),
                keys: vec!["Enter".to_string()],
            },
        ]
    );
}

#[test]
fn rejects_unmapped_zellij_control_input_explicitly() {
    let (attached, snapshot, _first_tab, _second_tab, terminal_pane, _plugin_pane) =
        sample_attached_session();

    let error = attached
        .dispatch_actions(
            &snapshot,
            MuxCommand::SendInput(SendInputSpec {
                pane_id: terminal_pane,
                data: "\u{0002}".to_string(),
                client_event_id: None,
            }),
        )
        .expect_err("unmapped control input should stay explicit");

    assert!(error.to_string().contains("control character"));
}

#[test]
fn maps_paste_to_target_terminal_pane() {
    let (attached, snapshot, _first_tab, _second_tab, terminal_pane, _plugin_pane) =
        sample_attached_session();

    let actions = attached
        .dispatch_actions(
            &snapshot,
            MuxCommand::SendPaste(SendPasteSpec {
                pane_id: terminal_pane,
                data: "hello\nworld".to_string(),
                client_event_id: None,
            }),
        )
        .expect("send-paste should map");

    assert_eq!(
        actions,
        vec![ZellijAction::Paste {
            pane_ref: "terminal_1".to_string(),
            text: "hello\nworld".to_string(),
        }]
    );
}

#[test]
fn rejects_plugin_input_writes() {
    let (attached, snapshot, _first_tab, _second_tab, _terminal_pane, plugin_pane) =
        sample_attached_session();

    let error = attached
        .dispatch_actions(
            &snapshot,
            MuxCommand::SendInput(SendInputSpec {
                pane_id: plugin_pane,
                data: "hello".to_string(),
                client_event_id: None,
            }),
        )
        .expect_err("plugin input should fail");

    assert_eq!(error.kind, BackendErrorKind::Unsupported);
    assert_eq!(error.degraded_reason, Some(DegradedModeReason::UnsupportedByBackend));
}

#[test]
fn rejects_closing_last_foreign_tab() {
    let (attached, snapshot, first_tab, _pane) = single_tab_attached_session();

    let error = attached
        .dispatch_actions(&snapshot, MuxCommand::CloseTab { tab_id: first_tab })
        .expect_err("closing the last tab should fail");

    assert_eq!(error.kind, BackendErrorKind::Unsupported);
    assert_eq!(error.degraded_reason, Some(DegradedModeReason::UnsupportedByBackend));
}

#[test]
fn rejects_closing_last_pane_in_tab() {
    let (attached, snapshot, _first_tab, pane_id) = single_tab_attached_session();

    let error = attached
        .dispatch_actions(&snapshot, MuxCommand::ClosePane { pane_id })
        .expect_err("closing the last pane should fail");

    assert_eq!(error.kind, BackendErrorKind::Unsupported);
    assert_eq!(error.degraded_reason, Some(DegradedModeReason::UnsupportedByBackend));
}

fn sample_attached_session() -> (
    ZellijAttachedSession,
    super::ZellijSessionSnapshot,
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

fn single_tab_attached_session() -> (
    ZellijAttachedSession,
    super::ZellijSessionSnapshot,
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
