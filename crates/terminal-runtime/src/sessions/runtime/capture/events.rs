use terminal_domain::{PaneId, RouteAuthority, SessionRoute};
use terminal_persistence::{
    BackendCapabilityReportInput, HistoryGapEventInput, ScreenSnapshotEventInput,
    SqliteSessionStore, TopologySnapshotEventInput,
};
use terminal_projection::{ScreenSnapshot, TopologySnapshot};

use crate::registry::SessionDescriptor;

pub(super) fn persist_backend_capability_report(
    persistence: &SqliteSessionStore,
    descriptor: &SessionDescriptor,
    capture_strategy: &str,
    capture_semantics: &str,
    command_boundary_confidence: &str,
    evidence_reason: &str,
) {
    let input = BackendCapabilityReportInput {
        id: None,
        session_id: Some(descriptor.session_id.0.to_string()),
        backend_kind: format!("{:?}", descriptor.route.backend).to_lowercase(),
        backend_version: None,
        backend_binary_path_hash: None,
        route_kind: persistence_route_kind(&descriptor.route),
        probe_status: "passed".to_string(),
        capture_strategy: capture_strategy.to_string(),
        capture_semantics: capture_semantics.to_string(),
        can_preserve_process_when_live: false,
        can_capture_scrollback: false,
        command_boundary_confidence: command_boundary_confidence.to_string(),
        evidence: Some(serde_json::json!({
            "source": "runtime_v2_history_capture",
            "reason": evidence_reason,
            "session_id": descriptor.session_id.0.to_string(),
            "backend": format!("{:?}", descriptor.route.backend).to_lowercase(),
        })),
        expires_at_ms: None,
    };
    let store = persistence.clone();
    let _ = tokio::task::spawn_blocking(move || {
        if let Err(error) = store.record_v2_backend_capability_report(input) {
            eprintln!("terminal-runtime: failed to persist v2 backend capability report - {error}");
        }
    });
}

pub(super) async fn persist_history_gap(
    persistence: &SqliteSessionStore,
    descriptor: &SessionDescriptor,
    tab_id: Option<String>,
    rows: Option<i32>,
    cols: Option<i32>,
    pane_id: PaneId,
    skipped_events: u64,
) {
    let input = HistoryGapEventInput {
        session_id: descriptor.session_id.0.to_string(),
        route: descriptor.route.clone(),
        title: descriptor.title.clone(),
        launch: descriptor.launch.clone(),
        pane_id: pane_id.0.to_string(),
        tab_id,
        rows,
        cols,
        skipped_events,
        estimated_dropped_bytes: None,
        reason: "raw_output_receiver_lagged".to_string(),
        occurred_at_ms: None,
    };
    let store = persistence.clone();
    match tokio::task::spawn_blocking(move || store.record_v2_history_gap(input)).await {
        Ok(Ok(())) => {}
        Ok(Err(error)) => {
            eprintln!("terminal-runtime: failed to persist v2 history gap - {error}");
        }
        Err(error) => {
            eprintln!("terminal-runtime: v2 history gap persistence task failed - {error}");
        }
    }
}

pub(super) async fn persist_topology_snapshot(
    persistence: &SqliteSessionStore,
    descriptor: &SessionDescriptor,
    topology: TopologySnapshot,
) {
    let input = TopologySnapshotEventInput {
        session_id: descriptor.session_id.0.to_string(),
        route: descriptor.route.clone(),
        title: descriptor.title.clone(),
        launch: descriptor.launch.clone(),
        topology,
    };
    let store = persistence.clone();
    match tokio::task::spawn_blocking(move || store.record_v2_topology_snapshot(input)).await {
        Ok(Ok(())) => {}
        Ok(Err(error)) => {
            eprintln!("terminal-runtime: failed to persist v2 topology snapshot - {error}");
        }
        Err(error) => {
            eprintln!("terminal-runtime: v2 topology snapshot persistence task failed - {error}");
        }
    }
}

pub(super) async fn persist_screen_snapshot(
    persistence: &SqliteSessionStore,
    descriptor: &SessionDescriptor,
    tab_id: Option<String>,
    screen: ScreenSnapshot,
) {
    let input = ScreenSnapshotEventInput {
        session_id: descriptor.session_id.0.to_string(),
        route: descriptor.route.clone(),
        title: descriptor.title.clone(),
        launch: descriptor.launch.clone(),
        tab_id,
        screen,
        buffer_kind: Some("normal".to_string()),
        capture_semantics: Some("rendered_plaintext_snapshot".to_string()),
    };
    let store = persistence.clone();
    match tokio::task::spawn_blocking(move || store.record_v2_screen_snapshot(input)).await {
        Ok(Ok(())) => {}
        Ok(Err(error)) => {
            eprintln!("terminal-runtime: failed to persist v2 screen snapshot - {error}");
        }
        Err(error) => {
            eprintln!("terminal-runtime: v2 screen snapshot persistence task failed - {error}");
        }
    }
}

fn persistence_route_kind(route: &SessionRoute) -> String {
    match route.authority {
        RouteAuthority::LocalDaemon => "local_daemon".to_string(),
        RouteAuthority::ImportedForeign => "imported_foreign".to_string(),
    }
}
