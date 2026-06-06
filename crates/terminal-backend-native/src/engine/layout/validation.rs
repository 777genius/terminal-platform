use std::collections::HashSet;

use terminal_backend_api::BackendError;
use terminal_domain::PaneId;
use terminal_mux_domain::PaneTreeNode;

use super::super::model::NativeTabRuntime;

pub(in crate::engine) fn validate_layout_override(
    tab: &NativeTabRuntime,
    root: &PaneTreeNode,
) -> Result<(), BackendError> {
    let current_panes: HashSet<_> = tab.pane_ids().into_iter().collect();
    let requested_panes = collect_snapshot_pane_ids(root);
    let requested_unique: HashSet<_> = requested_panes.iter().copied().collect();

    if requested_panes.len() != requested_unique.len() {
        return Err(BackendError::invalid_input("layout override contains duplicate pane ids"));
    }
    if current_panes != requested_unique {
        return Err(BackendError::invalid_input(
            "layout override must preserve the exact pane set for the target tab",
        ));
    }

    Ok(())
}

fn collect_snapshot_pane_ids(root: &PaneTreeNode) -> Vec<PaneId> {
    let mut pane_ids = Vec::new();
    collect_snapshot_pane_ids_inner(root, &mut pane_ids);
    pane_ids
}

fn collect_snapshot_pane_ids_inner(root: &PaneTreeNode, pane_ids: &mut Vec<PaneId>) {
    match root {
        PaneTreeNode::Leaf { pane_id } => pane_ids.push(*pane_id),
        PaneTreeNode::Split(split) => {
            collect_snapshot_pane_ids_inner(&split.first, pane_ids);
            collect_snapshot_pane_ids_inner(&split.second, pane_ids);
        }
    }
}
