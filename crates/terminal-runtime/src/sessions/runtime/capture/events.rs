use std::sync::Arc;

use terminal_domain::{PaneId, RouteAuthority, SessionId, SessionRoute};
use terminal_persistence::{
    BackendCapabilityReportInput, HistoryGapEventInput, ScreenSnapshotEventInput,
    SqliteSessionStore, TerminalPersistenceV2Error, TopologySnapshotEventInput,
};
use terminal_projection::{
    ScreenSnapshot, SessionHealthReason, SessionHealthSnapshot, TopologySnapshot,
};

use crate::registry::{SessionDescriptor, SessionRegistry};

#[derive(Clone)]
pub(super) struct CapturePersistenceDiagnostics {
    registry: Arc<dyn SessionRegistry>,
    session_id: SessionId,
}

impl CapturePersistenceDiagnostics {
    pub(super) fn new(registry: Arc<dyn SessionRegistry>, session_id: SessionId) -> Self {
        Self { registry, session_id }
    }

    pub(super) fn record_failure(&self, operation: &str, error: &TerminalPersistenceV2Error) {
        let detail = format!("terminal history persistence failed during {operation} - {error}");
        eprintln!("terminal-runtime: {detail}");
        self.registry.update_health(
            self.session_id,
            SessionHealthSnapshot::degraded(
                self.session_id,
                SessionHealthReason::HistoryPersistenceFault,
                detail,
            ),
        );
    }

    pub(super) fn record_task_failure(&self, operation: &str, error: &tokio::task::JoinError) {
        let detail =
            format!("terminal history persistence task failed during {operation} - {error}");
        eprintln!("terminal-runtime: {detail}");
        self.registry.update_health(
            self.session_id,
            SessionHealthSnapshot::degraded(
                self.session_id,
                SessionHealthReason::HistoryPersistenceFault,
                detail,
            ),
        );
    }
}

pub(super) fn persist_backend_capability_report(
    persistence: &SqliteSessionStore,
    diagnostics: &CapturePersistenceDiagnostics,
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
    let diagnostics = diagnostics.clone();
    tokio::spawn(async move {
        match tokio::task::spawn_blocking(move || store.record_v2_backend_capability_report(input))
            .await
        {
            Ok(Ok(_report_id)) => {}
            Ok(Err(error)) => {
                diagnostics.record_failure("backend capability report", &error);
            }
            Err(error) => {
                diagnostics.record_task_failure("backend capability report", &error);
            }
        }
    });
}

pub(super) async fn persist_history_gap(
    persistence: &SqliteSessionStore,
    diagnostics: &CapturePersistenceDiagnostics,
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
            diagnostics.record_failure("history gap", &error);
        }
        Err(error) => {
            diagnostics.record_task_failure("history gap", &error);
        }
    }
}

pub(super) async fn persist_topology_snapshot(
    persistence: &SqliteSessionStore,
    diagnostics: &CapturePersistenceDiagnostics,
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
            diagnostics.record_failure("topology snapshot", &error);
        }
        Err(error) => {
            diagnostics.record_task_failure("topology snapshot", &error);
        }
    }
}

pub(super) async fn persist_screen_snapshot(
    persistence: &SqliteSessionStore,
    diagnostics: &CapturePersistenceDiagnostics,
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
            diagnostics.record_failure("screen snapshot", &error);
        }
        Err(error) => {
            diagnostics.record_task_failure("screen snapshot", &error);
        }
    }
}

fn persistence_route_kind(route: &SessionRoute) -> String {
    match route.authority {
        RouteAuthority::LocalDaemon => "local_daemon".to_string(),
        RouteAuthority::ImportedForeign => "imported_foreign".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use terminal_domain::{BackendKind, RouteAuthority};
    use terminal_projection::{SessionHealthPhase, SessionHealthReason};

    use super::*;
    use crate::registry::{InMemorySessionRegistry, SessionRegistry};

    #[test]
    fn capture_persistence_failure_marks_session_health_degraded() {
        let registry = Arc::new(InMemorySessionRegistry::default());
        let session_id = SessionId::new();
        registry.insert(SessionDescriptor {
            session_id,
            route: SessionRoute {
                backend: BackendKind::Native,
                authority: RouteAuthority::LocalDaemon,
                external: None,
            },
            title: Some("shell".to_string()),
            launch: None,
            health: SessionHealthSnapshot::ready(session_id),
        });
        let diagnostics = CapturePersistenceDiagnostics::new(registry.clone(), session_id);

        diagnostics.record_failure(
            "screen snapshot",
            &TerminalPersistenceV2Error::InvalidData("sqlite full".to_string()),
        );

        let health = registry.get(session_id).expect("session should exist").health;
        assert_eq!(health.phase, SessionHealthPhase::Degraded);
        assert_eq!(health.reason, Some(SessionHealthReason::HistoryPersistenceFault));
        assert!(health.detail.as_deref().is_some_and(
            |detail| detail.contains("screen snapshot") && detail.contains("sqlite full")
        ));
    }
}
