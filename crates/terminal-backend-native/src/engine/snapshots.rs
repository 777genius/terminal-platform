use terminal_backend_api::BackendError;
use terminal_domain::PaneId;
use terminal_mux_domain::TabSnapshot;
use terminal_projection::{ScreenDelta, ScreenSnapshot, TopologySnapshot};

use super::NativeSessionEngine;

impl NativeSessionEngine {
    pub(crate) fn topology_snapshot(&self) -> Result<TopologySnapshot, BackendError> {
        let state = self.lock_state()?;

        Ok(TopologySnapshot {
            session_id: self.session_id,
            backend_kind: terminal_domain::BackendKind::Native,
            focused_tab: Some(state.focused_tab),
            tabs: state
                .tabs
                .iter()
                .map(|tab| TabSnapshot {
                    tab_id: tab.tab_id,
                    title: tab.title.clone(),
                    root: tab.root.snapshot(),
                    focused_pane: Some(tab.focused_pane),
                })
                .collect(),
        })
    }

    pub(crate) fn screen_snapshot(&self, pane_id: PaneId) -> Result<ScreenSnapshot, BackendError> {
        let state = self.lock_state()?;
        let (tab, pane) = state
            .tabs
            .iter()
            .find_map(|tab| tab.pane(pane_id).map(|pane| (tab, pane)))
            .ok_or_else(|| BackendError::not_found(format!("unknown pane {pane_id:?}")))?;

        pane.render_snapshot(tab.title.clone().or_else(|| state.summary.title.clone()))
    }

    pub(crate) fn screen_delta(
        &self,
        pane_id: PaneId,
        from_sequence: u64,
    ) -> Result<ScreenDelta, BackendError> {
        let state = self.lock_state()?;
        let (tab, pane) = state
            .tabs
            .iter()
            .find_map(|tab| tab.pane(pane_id).map(|pane| (tab, pane)))
            .ok_or_else(|| BackendError::not_found(format!("unknown pane {pane_id:?}")))?;

        pane.screen_delta(tab.title.clone().or_else(|| state.summary.title.clone()), from_sequence)
    }
}
