use std::collections::BTreeMap;

#[cfg(unix)]
use terminal_backend_api::{BackendScope, MuxBackendPort};
use terminal_domain::{RouteAuthority, SessionRoute};
use terminal_mux_domain::{PaneTreeNode, SplitDirection};

#[cfg(unix)]
use super::TmuxBackend;
use super::{TMUX_ROUTE_NAMESPACE, TmuxTarget, fallback_tree, parse_tmux_layout, tmux_split_flag};

#[test]
fn roundtrips_tmux_route_target() {
    let target = TmuxTarget {
        socket_name: Some("test-socket".to_string()),
        session_name: "workspace".to_string(),
    };

    let route = target.route();
    let decoded = TmuxTarget::from_route(&route).expect("route should decode");

    assert_eq!(route.backend, terminal_domain::BackendKind::Tmux);
    assert_eq!(route.authority, RouteAuthority::ImportedForeign);
    assert_eq!(decoded.socket_name.as_deref(), Some("test-socket"));
    assert_eq!(decoded.session_name, "workspace");
}

#[test]
fn rejects_invalid_tmux_route_namespace() {
    let route = SessionRoute {
        backend: terminal_domain::BackendKind::Tmux,
        authority: RouteAuthority::ImportedForeign,
        external: Some(terminal_domain::ExternalSessionRef {
            namespace: "other".to_string(),
            value: "session=workspace".to_string(),
        }),
    };

    let error = TmuxTarget::from_route(&route).expect_err("route should fail");
    assert_eq!(error.kind, terminal_backend_api::BackendErrorKind::InvalidInput);
}

#[test]
fn parses_nested_tmux_layout() {
    let pane_ids = BTreeMap::from([
        (0_u32, terminal_domain::PaneId::new()),
        (1_u32, terminal_domain::PaneId::new()),
        (2_u32, terminal_domain::PaneId::new()),
    ]);
    let root = parse_tmux_layout(
        "bb62,159x48,0,0{79x48,0,0,0,79x23,80,0[79x11,80,0,1,79x11,80,12,2]}",
        &pane_ids.into_iter().collect(),
    )
    .expect("layout should parse");

    match root {
        PaneTreeNode::Split(_) => {}
        other => panic!("unexpected layout root: {other:?}"),
    }
}

#[test]
fn builds_fallback_tree_for_multiple_panes() {
    let pane_a = terminal_domain::PaneId::new();
    let pane_b = terminal_domain::PaneId::new();
    let pane_c = terminal_domain::PaneId::new();
    let root = fallback_tree([pane_a, pane_b, pane_c].into_iter());

    match root {
        PaneTreeNode::Split(_) => {}
        other => panic!("unexpected fallback root: {other:?}"),
    }
}

#[test]
fn exported_namespace_stays_stable() {
    assert_eq!(TMUX_ROUTE_NAMESPACE, "tmux_target");
}

#[test]
fn maps_split_direction_to_tmux_flags_consistently() {
    assert_eq!(tmux_split_flag(SplitDirection::Horizontal), "-v");
    assert_eq!(tmux_split_flag(SplitDirection::Vertical), "-h");
}

#[cfg(unix)]
#[tokio::test(flavor = "current_thread")]
async fn discover_sessions_returns_empty_when_tmux_server_is_absent() {
    let backend = TmuxBackend::with_socket_name(format!(
        "terminal-platform-no-server-{}",
        uuid::Uuid::new_v4()
    ));

    let discovered = backend
        .discover_sessions(BackendScope::CurrentUser)
        .await
        .expect("missing tmux server should look like an empty discovery set");

    assert!(discovered.is_empty());
}

#[cfg(unix)]
#[tokio::test(flavor = "current_thread")]
async fn tmux_capabilities_advertise_rich_screen_surface() {
    let backend =
        TmuxBackend::with_socket_name(format!("terminal-platform-caps-{}", uuid::Uuid::new_v4()));

    let capabilities = backend.capabilities().await.expect("tmux capabilities should load");

    assert!(capabilities.rendered_viewport_snapshot);
    assert!(!capabilities.raw_output_stream);
    assert!(capabilities.rich_screen_surface);
}
