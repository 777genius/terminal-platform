use terminal_domain::{RouteAuthority, SessionRoute};

use crate::{
    ZELLIJ_ROUTE_NAMESPACE, ZellijTabRow, ZellijTarget, parse_panes_json, parse_tabs_json,
};

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
