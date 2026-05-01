use terminal_domain::PaneId;
use terminal_mux_domain::{PaneSplit, PaneTreeNode, SplitDirection};

use super::{
    super::{
        DEFAULT_SPLIT_RATIO_BPS,
        model::{NativePaneLayoutNode, NativePaneLayoutSplit, NativeTabRuntime},
    },
    geometry::partition_dimension_by_ratio,
};

impl NativeTabRuntime {
    pub(in crate::engine) fn pane(
        &self,
        pane_id: PaneId,
    ) -> Option<&super::super::model::NativePaneRuntime> {
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

    pub(in crate::engine) fn contains_pane(&self, target: PaneId) -> bool {
        match self {
            Self::Leaf { pane_id } => *pane_id == target,
            Self::Split(split) => {
                split.first.contains_pane(target) || split.second.contains_pane(target)
            }
        }
    }

    pub(in crate::engine) fn pane_ids(&self) -> Vec<PaneId> {
        let mut pane_ids = Vec::new();
        self.collect_pane_ids(&mut pane_ids);
        pane_ids
    }

    pub(in crate::engine) fn path_has_direction(
        &self,
        target: PaneId,
        direction: SplitDirection,
    ) -> bool {
        match self {
            Self::Leaf { .. } => false,
            Self::Split(split) => {
                let first_contains = split.first.contains_pane(target);
                let second_contains = split.second.contains_pane(target);
                if !first_contains && !second_contains {
                    return false;
                }

                if split.direction == direction {
                    true
                } else if first_contains {
                    split.first.path_has_direction(target, direction)
                } else {
                    split.second.path_has_direction(target, direction)
                }
            }
        }
    }

    pub(in crate::engine) fn first_pane_id(&self) -> Option<PaneId> {
        match self {
            Self::Leaf { pane_id } => Some(*pane_id),
            Self::Split(split) => {
                split.first.first_pane_id().or_else(|| split.second.first_pane_id())
            }
        }
    }

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

    fn collect_pane_ids(&self, pane_ids: &mut Vec<PaneId>) {
        match self {
            Self::Leaf { pane_id } => pane_ids.push(*pane_id),
            Self::Split(split) => {
                split.first.collect_pane_ids(pane_ids);
                split.second.collect_pane_ids(pane_ids);
            }
        }
    }
}

impl NativePaneLayoutSplit {
    pub(super) fn partition(&self, rows: u16, cols: u16) -> ((u16, u16), (u16, u16)) {
        match self.direction {
            SplitDirection::Vertical => {
                let (first_cols, second_cols) = partition_dimension_by_ratio(cols, self.ratio_bps);
                ((rows, first_cols), (rows, second_cols))
            }
            SplitDirection::Horizontal => {
                let (first_rows, second_rows) = partition_dimension_by_ratio(rows, self.ratio_bps);
                ((first_rows, cols), (second_rows, cols))
            }
        }
    }
}
