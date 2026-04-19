use serde::{Deserialize, Serialize};

use terminal_domain::{PaneId, TabId};

use crate::PaneTreeNode;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TabSnapshot {
    pub tab_id: TabId,
    pub title: Option<String>,
    pub root: PaneTreeNode,
    pub focused_pane: Option<PaneId>,
}
