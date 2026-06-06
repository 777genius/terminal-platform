use std::collections::HashMap;

use terminal_domain::PaneId;
use terminal_mux_domain::PaneTreeNode;

#[must_use]
pub fn parse_layout(input: &str) -> Option<PaneTreeNode> {
    let pane_ids: HashMap<u32, PaneId> = input
        .split(|ch: char| !ch.is_ascii_digit())
        .filter(|token| !token.is_empty())
        .filter_map(|token| token.parse::<u32>().ok())
        .map(|pane_id| (pane_id, PaneId::new()))
        .collect();

    crate::layout::parse_tmux_layout(input, &pane_ids)
}
