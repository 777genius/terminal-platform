use std::collections::HashMap;

use terminal_domain::{PaneId, TabId};
use terminal_projection::TopologySnapshot;

#[derive(Clone)]
pub(crate) struct ZellijSessionSnapshot {
    pub(crate) topology: TopologySnapshot,
    pub(crate) tab_targets: HashMap<TabId, ZellijTabTarget>,
    pub(crate) pane_targets: HashMap<PaneId, ZellijPaneTarget>,
}

impl ZellijSessionSnapshot {
    pub(crate) fn focused_backend_tab_id(&self) -> Option<u32> {
        self.topology
            .focused_tab
            .and_then(|tab_id| self.tab_targets.get(&tab_id))
            .map(|tab| tab.backend_tab_id)
    }

    pub(crate) fn tab_exists(&self, backend_tab_id: u32) -> bool {
        self.tab_targets.values().any(|tab| tab.backend_tab_id == backend_tab_id)
    }

    pub(crate) fn tab_title(&self, backend_tab_id: u32) -> Option<&str> {
        self.tab_targets
            .values()
            .find(|tab| tab.backend_tab_id == backend_tab_id)
            .and_then(|tab| tab.title.as_deref())
    }

    pub(crate) fn settle_summary(&self) -> String {
        let mut tabs: Vec<_> = self
            .tab_targets
            .values()
            .map(|tab| {
                format!(
                    "{}:{}:{}",
                    tab.backend_tab_id,
                    tab.display_index,
                    tab.title.as_deref().unwrap_or("<untitled>")
                )
            })
            .collect();
        tabs.sort();
        format!(
            "focused_backend_tab_id={:?}; tabs=[{}]",
            self.focused_backend_tab_id(),
            tabs.join(",")
        )
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ZellijPaneTarget {
    pub(crate) backend_ref: String,
    pub(crate) kind: ZellijPaneKind,
    pub(crate) title: Option<String>,
    pub(crate) rows: u16,
    pub(crate) cols: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ZellijPaneKind {
    Terminal,
    Plugin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ZellijTabTarget {
    pub(crate) backend_tab_id: u32,
    pub(crate) position: u32,
    pub(crate) display_index: u32,
    pub(crate) title: Option<String>,
}
