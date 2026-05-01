use std::collections::HashMap;

use terminal_backend_api::BackendError;
use terminal_domain::{BackendKind, PaneId, SessionId, TabId};
use terminal_mux_domain::{PaneSplit, PaneTreeNode, SplitDirection, TabSnapshot};
use terminal_projection::TopologySnapshot;
use uuid::Uuid;

use crate::{
    rows::{ZellijPaneRow, ZellijTabRow},
    target::ZellijTarget,
};

#[derive(Clone)]
pub(crate) struct ZellijSessionSnapshot {
    pub(crate) topology: TopologySnapshot,
    pub(crate) tab_targets: HashMap<TabId, ZellijTabTarget>,
    pub(crate) pane_targets: HashMap<PaneId, ZellijPaneTarget>,
}

impl ZellijSessionSnapshot {
    pub(crate) fn focused_backend_tab_id(&self) -> Option<u32> {
        self.topology
            .focused_tab
            .and_then(|tab_id| self.tab_targets.get(&tab_id))
            .map(|tab| tab.backend_tab_id)
    }

    pub(crate) fn tab_exists(&self, backend_tab_id: u32) -> bool {
        self.tab_targets.values().any(|tab| tab.backend_tab_id == backend_tab_id)
    }

    pub(crate) fn tab_title(&self, backend_tab_id: u32) -> Option<&str> {
        self.tab_targets
            .values()
            .find(|tab| tab.backend_tab_id == backend_tab_id)
            .and_then(|tab| tab.title.as_deref())
    }

    pub(crate) fn settle_summary(&self) -> String {
        let mut tabs: Vec<_> = self
            .tab_targets
            .values()
            .map(|tab| {
                format!(
                    "{}:{}:{}",
                    tab.backend_tab_id,
                    tab.display_index,
                    tab.title.as_deref().unwrap_or("<untitled>")
                )
            })
            .collect();
        tabs.sort();
        format!(
            "focused_backend_tab_id={:?}; tabs=[{}]",
            self.focused_backend_tab_id(),
            tabs.join(",")
        )
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ZellijPaneTarget {
    pub(crate) backend_ref: String,
    pub(crate) kind: ZellijPaneKind,
    pub(crate) title: Option<String>,
    pub(crate) rows: u16,
    pub(crate) cols: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ZellijPaneKind {
    Terminal,
    Plugin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ZellijTabTarget {
    pub(crate) backend_tab_id: u32,
    pub(crate) position: u32,
    pub(crate) display_index: u32,
    pub(crate) title: Option<String>,
}

pub(crate) fn build_session_snapshot(
    session_id: SessionId,
    target: &ZellijTarget,
    tabs: &[ZellijTabRow],
    panes: &[ZellijPaneRow],
) -> Result<ZellijSessionSnapshot, BackendError> {
    let mut tabs = tabs.to_vec();
    tabs.sort_by_key(|tab| if tab.position == 0 { tab.tab_id } else { tab.position });

    let mut tab_targets = HashMap::new();
    let mut pane_targets = HashMap::new();
    let mut topology_tabs = Vec::new();
    let mut focused_tab = None;
    let focused_tab_from_pane = panes.iter().find(|pane| pane.is_focused).map(|pane| pane.tab_id);

    for (ordinal, tab) in tabs.into_iter().enumerate() {
        let position = if tab.position == 0 { ordinal as u32 + 1 } else { tab.position };
        let mut tab_panes: Vec<ZellijPaneRow> = panes
            .iter()
            .filter(|pane| pane.tab_id == tab.tab_id && !pane.is_floating)
            .cloned()
            .collect();
        if tab_panes.is_empty() {
            continue;
        }

        tab_panes.sort_by_key(|pane| (pane.pane_y, pane.pane_x, pane.id));
        let tab_id = deterministic_tab_id(target, tab.tab_id, position);
        let pane_ids: Vec<PaneId> = tab_panes
            .iter()
            .map(|pane| {
                let pane_id = deterministic_pane_id(target, tab.tab_id, &pane.backend_ref());
                pane_targets.insert(
                    pane_id,
                    ZellijPaneTarget {
                        backend_ref: pane.backend_ref(),
                        kind: if pane.is_plugin {
                            ZellijPaneKind::Plugin
                        } else {
                            ZellijPaneKind::Terminal
                        },
                        title: non_empty(&pane.title),
                        rows: pane.pane_rows,
                        cols: pane.pane_columns,
                    },
                );
                pane_id
            })
            .collect();
        let focused_pane = tab_panes
            .iter()
            .find(|pane| pane.is_focused)
            .map(|pane| deterministic_pane_id(target, tab.tab_id, &pane.backend_ref()))
            .or_else(|| pane_ids.first().copied());

        tab_targets.insert(
            tab_id,
            ZellijTabTarget {
                backend_tab_id: tab.tab_id,
                position,
                display_index: tab.position,
                title: non_empty(&tab.name),
            },
        );

        if focused_tab.is_none() && (tab.active || focused_tab_from_pane == Some(tab.tab_id)) {
            focused_tab = Some(tab_id);
        }

        topology_tabs.push((
            position,
            TabSnapshot {
                tab_id,
                title: non_empty(&tab.name),
                root: fallback_tree(pane_ids.into_iter()),
                focused_pane,
            },
        ));
    }

    topology_tabs.sort_by_key(|(position, _)| *position);
    let tabs: Vec<TabSnapshot> = topology_tabs.into_iter().map(|(_, tab)| tab).collect();
    if tabs.is_empty() {
        return Err(BackendError::not_found(format!(
            "zellij session '{}' exposed no importable panes",
            target.session_name
        )));
    }
    let focused_tab = focused_tab.or_else(|| tabs.first().map(|tab| tab.tab_id));

    Ok(ZellijSessionSnapshot {
        topology: TopologySnapshot {
            session_id,
            backend_kind: BackendKind::Zellij,
            tabs,
            focused_tab,
        },
        tab_targets,
        pane_targets,
    })
}

fn deterministic_tab_id(target: &ZellijTarget, backend_tab_id: u32, position: u32) -> TabId {
    deterministic_uuid(
        &format!(
            "terminal-platform/zellij/tab/{}/{}/{}",
            target.session_name, backend_tab_id, position
        ),
        TabId::from,
    )
}

fn deterministic_pane_id(target: &ZellijTarget, backend_tab_id: u32, backend_ref: &str) -> PaneId {
    deterministic_uuid(
        &format!(
            "terminal-platform/zellij/pane/{}/{}/{}",
            target.session_name, backend_tab_id, backend_ref
        ),
        PaneId::from,
    )
}

fn deterministic_uuid<T>(fingerprint: &str, construct: fn(Uuid) -> T) -> T {
    construct(Uuid::new_v5(&Uuid::NAMESPACE_URL, fingerprint.as_bytes()))
}

fn fallback_tree(mut pane_ids: impl Iterator<Item = PaneId>) -> PaneTreeNode {
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

pub(crate) fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

pub(crate) fn tab_contains_pane(tab: &TabSnapshot, pane_id: PaneId) -> bool {
    collect_pane_ids(&tab.root).into_iter().any(|candidate| candidate == pane_id)
}

pub(crate) fn collect_pane_ids(root: &PaneTreeNode) -> Vec<PaneId> {
    let mut pane_ids = Vec::new();
    collect_pane_ids_inner(root, &mut pane_ids);
    pane_ids
}

pub(crate) fn collect_pane_ids_inner(root: &PaneTreeNode, pane_ids: &mut Vec<PaneId>) {
    match root {
        PaneTreeNode::Leaf { pane_id } => pane_ids.push(*pane_id),
        PaneTreeNode::Split(split) => {
            collect_pane_ids_inner(&split.first, pane_ids);
            collect_pane_ids_inner(&split.second, pane_ids);
        }
    }
}
