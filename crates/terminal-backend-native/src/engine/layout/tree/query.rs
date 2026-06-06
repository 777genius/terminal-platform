use terminal_domain::PaneId;
use terminal_mux_domain::SplitDirection;

use crate::engine::model::NativePaneLayoutNode;

impl NativePaneLayoutNode {
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
