use terminal_backend_api::BackendError;
use terminal_domain::{BackendKind, DegradedModeReason, saved_session_compatibility};
use terminal_persistence::SavedNativeSession;

pub(super) fn validate_native_restore(saved: &SavedNativeSession) -> Result<(), BackendError> {
    let compatibility = saved_session_compatibility(&saved.manifest);
    if !compatibility.can_restore {
        return Err(BackendError::unsupported(
            format!(
                "saved session manifest is not restore-compatible - {:?}",
                compatibility.status
            ),
            DegradedModeReason::SavedSessionIncompatible,
        ));
    }

    if saved.route.backend != BackendKind::Native {
        return Err(BackendError::unsupported(
            "saved-session restore currently supports native runtime sessions only; imported multiplexor sessions do not claim saved-session replay guarantees",
            DegradedModeReason::UnsupportedByBackend,
        ));
    }

    if saved.topology.tabs.is_empty() {
        return Err(BackendError::invalid_input("saved native session has no tabs"));
    }

    Ok(())
}

pub(super) fn initial_restore_title(saved: &SavedNativeSession) -> Option<String> {
    saved.topology.tabs.first().and_then(|tab| tab.title.clone()).or(saved.title.clone())
}
