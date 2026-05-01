use terminal_backend_api::BackendSessionSummary;
use terminal_domain::{SessionId, saved_session_compatibility};
use terminal_protocol::{RestoreSavedSessionResponse, SavedSessionRecord, SavedSessionSummary};

use crate::application::{RuntimeSavedSessionRecord, RuntimeSavedSessionSummary};

use super::semantics::{saved_session_restore_semantics, saved_session_restore_semantics_v2};

pub fn map_saved_session_summary(session: RuntimeSavedSessionSummary) -> SavedSessionSummary {
    let compatibility = saved_session_compatibility(&session.manifest);
    let restore_semantics =
        saved_session_restore_semantics(session.has_launch, session.restore_plan.as_ref());
    let restore_semantics_v2 = saved_session_restore_semantics_v2(
        session.session_id,
        None,
        &restore_semantics,
        session.restore_plan.as_ref(),
    );

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
        restore_semantics,
        restore_semantics_v2,
    }
}

pub fn map_saved_session_record(session: RuntimeSavedSessionRecord) -> SavedSessionRecord {
    let has_launch = session.launch.is_some();
    let compatibility = saved_session_compatibility(&session.manifest);
    let restore_semantics =
        saved_session_restore_semantics(has_launch, session.restore_plan.as_ref());
    let restore_semantics_v2 = saved_session_restore_semantics_v2(
        session.session_id,
        None,
        &restore_semantics,
        session.restore_plan.as_ref(),
    );

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
        restore_semantics,
        restore_semantics_v2,
    }
}

pub fn map_restore_saved_session_response(
    saved_session_id: SessionId,
    saved_session: &RuntimeSavedSessionRecord,
    restored_session: BackendSessionSummary,
) -> RestoreSavedSessionResponse {
    let restore_semantics = saved_session_restore_semantics(
        saved_session.launch.is_some(),
        saved_session.restore_plan.as_ref(),
    );
    let restore_semantics_v2 = saved_session_restore_semantics_v2(
        saved_session_id,
        Some(restored_session.session_id),
        &restore_semantics,
        saved_session.restore_plan.as_ref(),
    );
    RestoreSavedSessionResponse {
        saved_session_id,
        manifest: saved_session.manifest.clone(),
        compatibility: saved_session_compatibility(&saved_session.manifest),
        session: restored_session,
        restore_semantics,
        restore_semantics_v2,
    }
}
