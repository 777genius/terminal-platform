use terminal_domain::PaneId;
use terminal_mux_domain::SplitDirection;

use crate::engine::{
    DEFAULT_SPLIT_RATIO_BPS,
    model::{NativePaneLayoutNode, NativePaneLayoutSplit},
};

impl NativePaneLayoutNode {
    pub(in crate::engine) fn split_leaf(
        &mut self,
        target: PaneId,
        direction: SplitDirection,
        new_pane: PaneId,
    ) -> bool {
        match self {
            Self::Leaf { pane_id } if *pane_id == target => {
                let current_pane = *pane_id;
                *self = Self::Split(NativePaneLayoutSplit {
                    direction,
                    ratio_bps: DEFAULT_SPLIT_RATIO_BPS,
                    first: Box::new(Self::Leaf { pane_id: current_pane }),
                    second: Box::new(Self::Leaf { pane_id: new_pane }),
                });
                true
            }
            Self::Leaf { .. } => false,
            Self::Split(split) => {
                split.first.split_leaf(target, direction, new_pane)
                    || split.second.split_leaf(target, direction, new_pane)
            }
        }
    }

    pub(in crate::engine) fn remove_leaf(&self, target: PaneId) -> Option<Self> {
        match self {
            Self::Leaf { pane_id } => {
                (*pane_id != target).then_some(Self::Leaf { pane_id: *pane_id })
            }
            Self::Split(split) => {
                match (split.first.remove_leaf(target), split.second.remove_leaf(target)) {
                    (Some(first), Some(second)) => Some(Self::Split(NativePaneLayoutSplit {
                        direction: split.direction,
                        ratio_bps: split.ratio_bps,
                        first: Box::new(first),
                        second: Box::new(second),
                    })),
                    (Some(node), None) | (None, Some(node)) => Some(node),
                    (None, None) => None,
                }
            }
        }
    }
}
