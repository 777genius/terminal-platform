use crate::{prelude::*, target::TmuxTarget};

pub(crate) fn deterministic_tab_id(target: &TmuxTarget, window_id: &str) -> TabId {
    deterministic_uuid(
        &format!(
            "terminal-platform/tmux/tab/{:?}/{}/{}",
            target.socket_name, target.session_name, window_id
        ),
        TabId::from,
    )
}

pub(crate) fn deterministic_pane_id(target: &TmuxTarget, window_id: &str, pane_id: &str) -> PaneId {
    deterministic_uuid(
        &format!(
            "terminal-platform/tmux/pane/{:?}/{}/{}/{}",
            target.socket_name, target.session_name, window_id, pane_id
        ),
        PaneId::from,
    )
}

fn deterministic_uuid<T>(fingerprint: &str, construct: fn(Uuid) -> T) -> T {
    construct(Uuid::new_v5(&Uuid::NAMESPACE_URL, fingerprint.as_bytes()))
}

pub(crate) fn tmux_split_flag(direction: SplitDirection) -> &'static str {
    match direction {
        SplitDirection::Horizontal => "-v",
        SplitDirection::Vertical => "-h",
    }
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
