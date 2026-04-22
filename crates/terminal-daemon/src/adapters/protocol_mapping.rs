use terminal_backend_api::{BackendError, BackendErrorKind, BackendSessionSummary};
use terminal_domain::{SessionId, saved_session_compatibility};
use terminal_protocol::{
    ProtocolError, RestoreSavedSessionResponse, SavedSessionRecord, SavedSessionRestoreSemantics,
    SavedSessionSummary,
};

use crate::application::{RuntimeSavedSessionRecord, RuntimeSavedSessionSummary};

pub fn map_backend_error(error: BackendError) -> ProtocolError {
    let code = match error.kind {
        BackendErrorKind::Unsupported => "backend_unsupported",
        BackendErrorKind::NotFound => "backend_not_found",
        BackendErrorKind::InvalidInput => "backend_invalid_input",
        BackendErrorKind::Transport => "backend_transport",
        BackendErrorKind::Internal => "backend_internal",
    };
    let message = error.to_string();

    match error.degraded_reason {
        Some(degraded_reason) => {
            ProtocolError::with_degraded_reason(code, message, degraded_reason)
        }
        None => ProtocolError::new(code, message),
    }
}

pub fn map_saved_session_summary(session: RuntimeSavedSessionSummary) -> SavedSessionSummary {
    let compatibility = saved_session_compatibility(&session.manifest);

    SavedSessionSummary {
        session_id: session.session_id,
        route: session.route,
        title: session.title,
        saved_at_ms: session.saved_at_ms,
        manifest: session.manifest,
        compatibility,
        has_launch: session.has_launch,
        tab_count: session.tab_count,
        pane_count: session.pane_count,
        restore_semantics: saved_session_restore_semantics(session.has_launch),
    }
}

pub fn map_saved_session_record(session: RuntimeSavedSessionRecord) -> SavedSessionRecord {
    let has_launch = session.launch.is_some();
    let compatibility = saved_session_compatibility(&session.manifest);

    SavedSessionRecord {
        session_id: session.session_id,
        route: session.route,
        title: session.title,
        launch: session.launch,
        manifest: session.manifest,
        compatibility,
        topology: session.topology,
        screens: session.screens,
        saved_at_ms: session.saved_at_ms,
        restore_semantics: saved_session_restore_semantics(has_launch),
    }
}

pub fn map_restore_saved_session_response(
    saved_session_id: SessionId,
    saved_session: &RuntimeSavedSessionRecord,
    restored_session: BackendSessionSummary,
) -> RestoreSavedSessionResponse {
    RestoreSavedSessionResponse {
        saved_session_id,
        manifest: saved_session.manifest.clone(),
        compatibility: saved_session_compatibility(&saved_session.manifest),
        session: restored_session,
        restore_semantics: saved_session_restore_semantics(saved_session.launch.is_some()),
    }
}

fn saved_session_restore_semantics(has_launch: bool) -> SavedSessionRestoreSemantics {
    SavedSessionRestoreSemantics {
        restores_topology: true,
        restores_focus_state: true,
        restores_tab_titles: true,
        uses_saved_launch_spec: has_launch,
        replays_saved_screen_buffers: false,
        preserves_process_state: false,
    }
}
