use terminal_domain::PaneId;

use crate::engine::model::{NativePaneRuntime, NativeTabRuntime};

impl NativeTabRuntime {
    pub(in crate::engine) fn pane(&self, pane_id: PaneId) -> Option<&NativePaneRuntime> {
        self.panes.iter().find(|pane| pane.pane_id == pane_id)
    }

    pub(in crate::engine) fn pane_ids(&self) -> Vec<PaneId> {
        self.root.pane_ids()
    }

    pub(in crate::engine) fn contains_pane(&self, pane_id: PaneId) -> bool {
        self.root.contains_pane(pane_id)
    }

    pub(in crate::engine) fn first_pane_id(&self) -> Option<PaneId> {
        self.root.first_pane_id()
    }
}
