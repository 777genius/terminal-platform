use terminal_domain::PaneId;
use terminal_mux_domain::{PaneSplit, PaneTreeNode, SplitDirection, TabSnapshot};

pub(super) fn fallback_tree(mut pane_ids: impl Iterator<Item = PaneId>) -> PaneTreeNode {
    let first = pane_ids
        .next()
        .map(|pane_id| PaneTreeNode::Leaf { pane_id })
        .unwrap_or_else(|| PaneTreeNode::Leaf { pane_id: PaneId::new() });

    pane_ids.fold(first, |node, pane_id| {
        PaneTreeNode::Split(PaneSplit {
            direction: SplitDirection::Vertical,
            first: Box::new(node),
            second: Box::new(PaneTreeNode::Leaf { pane_id }),
        })
    })
}

pub(crate) fn tab_contains_pane(tab: &TabSnapshot, pane_id: PaneId) -> bool {
    collect_pane_ids(&tab.root).into_iter().any(|candidate| candidate == pane_id)
}

pub(crate) fn collect_pane_ids(root: &PaneTreeNode) -> Vec<PaneId> {
    let mut pane_ids = Vec::new();
    collect_pane_ids_inner(root, &mut pane_ids);
    pane_ids
}

fn collect_pane_ids_inner(root: &PaneTreeNode, pane_ids: &mut Vec<PaneId>) {
    match root {
        PaneTreeNode::Leaf { pane_id } => pane_ids.push(*pane_id),
        PaneTreeNode::Split(split) => {
            collect_pane_ids_inner(&split.first, pane_ids);
            collect_pane_ids_inner(&split.second, pane_ids);
        }
    }
}
