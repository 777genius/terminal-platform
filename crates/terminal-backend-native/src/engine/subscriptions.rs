use terminal_backend_api::{BackendError, BackendRawOutputEvent};
use terminal_domain::PaneId;
use tokio::sync::{broadcast, watch};

use super::NativeSessionEngine;

impl NativeSessionEngine {
    pub(crate) fn subscribe_topology(&self) -> watch::Receiver<u64> {
        self.topology_tick.subscribe()
    }

    pub(crate) fn subscribe_pane_surface(
        &self,
        pane_id: PaneId,
    ) -> Result<watch::Receiver<u64>, BackendError> {
        let state = self.lock_state()?;
        let pane = state
            .tabs
            .iter()
            .find_map(|tab| tab.pane(pane_id))
            .ok_or_else(|| BackendError::not_found(format!("unknown pane {pane_id:?}")))?;

        Ok(pane.surface_tick.subscribe())
    }

    pub(crate) fn subscribe_pane_raw_output(
        &self,
        pane_id: PaneId,
    ) -> Result<broadcast::Receiver<BackendRawOutputEvent>, BackendError> {
        let state = self.lock_state()?;
        let pane = state
            .tabs
            .iter()
            .find_map(|tab| tab.pane(pane_id))
            .ok_or_else(|| BackendError::not_found(format!("unknown pane {pane_id:?}")))?;

        Ok(pane.raw_output_tick.subscribe())
    }
}
