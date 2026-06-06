use terminal_protocol::{DaemonCapabilities, DaemonPhase, Handshake, ProtocolVersion};
use terminal_runtime::{RuntimeHandshake, RuntimePhase};

use crate::application::{RuntimeSavedSessionRecord, RuntimeSavedSessionSummary};

pub(super) fn map_runtime_handshake(handshake: RuntimeHandshake) -> Handshake {
    Handshake {
        protocol_version: ProtocolVersion {
            major: handshake.protocol_version.major,
            minor: handshake.protocol_version.minor,
        },
        binary_version: handshake.binary_version,
        daemon_phase: match handshake.daemon_phase {
            RuntimePhase::Starting => DaemonPhase::Starting,
            RuntimePhase::Ready => DaemonPhase::Ready,
            RuntimePhase::Degraded => DaemonPhase::Degraded,
        },
        capabilities: DaemonCapabilities {
            request_reply: handshake.capabilities.request_reply,
            topology_subscriptions: handshake.capabilities.topology_subscriptions,
            pane_subscriptions: handshake.capabilities.pane_subscriptions,
            backend_discovery: handshake.capabilities.backend_discovery,
            backend_capability_queries: handshake.capabilities.backend_capability_queries,
            saved_sessions: handshake.capabilities.saved_sessions,
            session_restore: handshake.capabilities.session_restore,
            degraded_error_reasons: handshake.capabilities.degraded_error_reasons,
            session_health: handshake.capabilities.session_health,
        },
        available_backends: handshake.available_backends,
        session_scope: handshake.session_scope,
    }
}

pub(super) fn map_saved_session_summary(
    session: terminal_persistence::SavedSessionSummary,
    restore_plan: Option<terminal_persistence::RestorePlan>,
) -> RuntimeSavedSessionSummary {
    RuntimeSavedSessionSummary {
        session_id: session.session_id,
        route: session.route,
        title: session.title,
        saved_at_ms: session.saved_at_ms,
        manifest: session.manifest,
        has_launch: session.has_launch,
        tab_count: session.tab_count,
        pane_count: session.pane_count,
        restore_plan,
    }
}

pub(super) fn map_saved_session_record(
    session: terminal_persistence::SavedNativeSession,
    restore_plan: Option<terminal_persistence::RestorePlan>,
) -> RuntimeSavedSessionRecord {
    RuntimeSavedSessionRecord {
        session_id: session.session_id,
        route: session.route,
        title: session.title,
        launch: session.launch,
        manifest: session.manifest,
        topology: session.topology,
        screens: session.screens,
        saved_at_ms: session.saved_at_ms,
        restore_plan,
    }
}
