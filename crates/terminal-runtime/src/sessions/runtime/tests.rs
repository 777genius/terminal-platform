use terminal_backend_api::{BackendError, MuxCommand, NewTabSpec};
use terminal_domain::{BackendKind, RouteAuthority, SessionId, SessionRoute, TabId};
use terminal_mux_domain::{PaneTreeNode, TabSnapshot};
use terminal_projection::{SessionHealthPhase, TopologySnapshot};

use super::session_health_from_attach_error;
use super::{command_updates_summary_title, saved_session_title, session_route_fingerprint};

#[test]
fn saved_session_title_prefers_focused_tab_title() {
    let tab_id = TabId::new();
    let topology = TopologySnapshot {
        session_id: SessionId::new(),
        backend_kind: BackendKind::Native,
        tabs: vec![TabSnapshot {
            tab_id,
            title: Some("logs".to_string()),
            root: PaneTreeNode::Leaf { pane_id: terminal_domain::PaneId::new() },
            focused_pane: None,
        }],
        focused_tab: Some(tab_id),
    };

    assert_eq!(
        saved_session_title(Some("fallback".to_string()), &topology),
        Some("logs".to_string())
    );
}

#[test]
fn command_title_refresh_tracks_only_tab_mutations() {
    assert!(command_updates_summary_title(&MuxCommand::NewTab(NewTabSpec::default())));
    assert!(!command_updates_summary_title(&MuxCommand::SaveSession));
}

#[test]
fn attach_error_maps_transport_failures_to_stale_health() {
    let session_id = SessionId::new();
    let health = session_health_from_attach_error(
        session_id,
        &BackendError::transport("connection dropped"),
    )
    .expect("transport error should map to health");

    assert_eq!(health.phase, SessionHealthPhase::Stale);
    assert!(health.invalidated);
}

#[test]
fn session_route_fingerprint_distinguishes_foreign_routes() {
    let route_a = SessionRoute {
        backend: BackendKind::Tmux,
        authority: RouteAuthority::ImportedForeign,
        external: Some(terminal_domain::ExternalSessionRef {
            namespace: "tmux_session".to_string(),
            value: "alpha".to_string(),
        }),
    };
    let route_b = SessionRoute {
        backend: BackendKind::Tmux,
        authority: RouteAuthority::ImportedForeign,
        external: Some(terminal_domain::ExternalSessionRef {
            namespace: "tmux_session".to_string(),
            value: "beta".to_string(),
        }),
    };

    assert_ne!(session_route_fingerprint(&route_a), session_route_fingerprint(&route_b));
}
