use serde::{Deserialize, Serialize};

use terminal_domain::PaneId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaneSplit {
    pub direction: SplitDirection,
    pub first: Box<PaneTreeNode>,
    pub second: Box<PaneTreeNode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PaneTreeNode {
    Leaf { pane_id: PaneId },
    Split(PaneSplit),
}
