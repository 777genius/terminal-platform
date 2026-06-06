use terminal_mux_domain::{PaneSplit, PaneTreeNode};

use crate::engine::{
    DEFAULT_SPLIT_RATIO_BPS,
    model::{NativePaneLayoutNode, NativePaneLayoutSplit},
};

impl NativePaneLayoutNode {
    pub(in crate::engine) fn from_snapshot(root: PaneTreeNode) -> Self {
        match root {
            PaneTreeNode::Leaf { pane_id } => Self::Leaf { pane_id },
            PaneTreeNode::Split(split) => Self::Split(NativePaneLayoutSplit {
                direction: split.direction,
                ratio_bps: DEFAULT_SPLIT_RATIO_BPS,
                first: Box::new(Self::from_snapshot(*split.first)),
                second: Box::new(Self::from_snapshot(*split.second)),
            }),
        }
    }

    pub(in crate::engine) fn snapshot(&self) -> PaneTreeNode {
        match self {
            Self::Leaf { pane_id } => PaneTreeNode::Leaf { pane_id: *pane_id },
            Self::Split(split) => PaneTreeNode::Split(PaneSplit {
                direction: split.direction,
                first: Box::new(split.first.snapshot()),
                second: Box::new(split.second.snapshot()),
            }),
        }
    }
}
