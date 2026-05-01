use crate::dto::{prelude::*, *};

use super::ids::parse_pane_id;

impl From<&SplitDirection> for NodeSplitDirection {
    fn from(value: &SplitDirection) -> Self {
        match value {
            SplitDirection::Horizontal => Self::Horizontal,
            SplitDirection::Vertical => Self::Vertical,
        }
    }
}

impl From<&NodeSplitDirection> for SplitDirection {
    fn from(value: &NodeSplitDirection) -> Self {
        match value {
            NodeSplitDirection::Horizontal => Self::Horizontal,
            NodeSplitDirection::Vertical => Self::Vertical,
        }
    }
}

impl From<&PaneSplit> for NodePaneSplit {
    fn from(value: &PaneSplit) -> Self {
        Self {
            direction: (&value.direction).into(),
            first: Box::new((&*value.first).into()),
            second: Box::new((&*value.second).into()),
        }
    }
}

impl From<&PaneTreeNode> for NodePaneTreeNode {
    fn from(value: &PaneTreeNode) -> Self {
        match value {
            PaneTreeNode::Leaf { pane_id } => Self::Leaf { pane_id: pane_id.0.to_string() },
            PaneTreeNode::Split(split) => Self::Split(split.into()),
        }
    }
}

impl TryFrom<&NodePaneTreeNode> for PaneTreeNode {
    type Error = ProtocolError;

    fn try_from(value: &NodePaneTreeNode) -> Result<Self, Self::Error> {
        match value {
            NodePaneTreeNode::Leaf { pane_id } => {
                Ok(Self::Leaf { pane_id: parse_pane_id(pane_id)? })
            }
            NodePaneTreeNode::Split(split) => Ok(Self::Split(PaneSplit {
                direction: (&split.direction).into(),
                first: Box::new((&*split.first).try_into()?),
                second: Box::new((&*split.second).try_into()?),
            })),
        }
    }
}
