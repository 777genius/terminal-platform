use crate::dto::{prelude::*, *};

impl From<&TabSnapshot> for NodeTabSnapshot {
    fn from(value: &TabSnapshot) -> Self {
        Self {
            tab_id: value.tab_id.0.to_string(),
            title: value.title.clone(),
            root: (&value.root).into(),
            focused_pane: value.focused_pane.map(|pane_id| pane_id.0.to_string()),
        }
    }
}

impl From<&TopologySnapshot> for NodeTopologySnapshot {
    fn from(value: &TopologySnapshot) -> Self {
        Self {
            session_id: value.session_id.0.to_string(),
            backend_kind: (&value.backend_kind).into(),
            tabs: value.tabs.iter().map(Into::into).collect(),
            focused_tab: value.focused_tab.map(|tab_id| tab_id.0.to_string()),
        }
    }
}

impl From<&MuxCommandResult> for NodeMuxCommandResult {
    fn from(value: &MuxCommandResult) -> Self {
        Self { changed: value.changed }
    }
}
