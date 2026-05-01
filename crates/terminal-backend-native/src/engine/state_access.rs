use terminal_backend_api::BackendError;
use terminal_domain::PaneId;

use super::{NativeSessionEngine, NativeSessionState, signals::bump_watch};

impl NativeSessionEngine {
    pub(super) fn finish_mutation(
        &self,
        state: &NativeSessionState,
        changed: bool,
        surface_updates: Vec<PaneId>,
    ) {
        if changed {
            bump_watch(&self.topology_tick);
        }
        for pane_id in surface_updates {
            if let Some(pane) = state.tabs.iter().find_map(|tab| tab.pane(pane_id)) {
                pane.mark_surface_dirty();
            }
        }
    }

    pub(super) fn lock_state(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, NativeSessionState>, BackendError> {
        self.state.lock().map_err(|_| BackendError::internal("native session state lock poisoned"))
    }
}
