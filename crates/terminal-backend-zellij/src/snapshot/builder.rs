use std::collections::HashMap;

use terminal_backend_api::BackendError;
use terminal_domain::{BackendKind, PaneId, SessionId};
use terminal_mux_domain::TabSnapshot;
use terminal_projection::TopologySnapshot;

use crate::{
    rows::{ZellijPaneRow, ZellijTabRow},
    target::ZellijTarget,
};

use super::{
    ids::{deterministic_pane_id, deterministic_tab_id},
    targets::{ZellijPaneKind, ZellijPaneTarget, ZellijSessionSnapshot, ZellijTabTarget},
    tree::fallback_tree,
};

pub(crate) fn build_session_snapshot(
    session_id: SessionId,
    target: &ZellijTarget,
    tabs: &[ZellijTabRow],
    panes: &[ZellijPaneRow],
) -> Result<ZellijSessionSnapshot, BackendError> {
    let mut tabs = sorted_tabs(tabs);
    let mut tab_targets = HashMap::new();
    let mut pane_targets = HashMap::new();
    let mut topology_tabs = Vec::new();
    let mut focused_tab = None;
    let focused_tab_from_pane = panes.iter().find(|pane| pane.is_focused).map(|pane| pane.tab_id);

    for (ordinal, tab) in tabs.drain(..).enumerate() {
        let position = if tab.position == 0 { ordinal as u32 + 1 } else { tab.position };
        let tab_panes = sorted_importable_panes(panes, tab.tab_id);
        if tab_panes.is_empty() {
            continue;
        }

        let tab_id = deterministic_tab_id(target, tab.tab_id, position);
        let pane_ids = map_pane_targets(target, &tab, &tab_panes, &mut pane_targets);
        let focused_pane = focused_pane_id(target, &tab, &tab_panes, &pane_ids);
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

fn sorted_tabs(tabs: &[ZellijTabRow]) -> Vec<ZellijTabRow> {
    let mut tabs = tabs.to_vec();
    tabs.sort_by_key(|tab| if tab.position == 0 { tab.tab_id } else { tab.position });
    tabs
}

fn sorted_importable_panes(panes: &[ZellijPaneRow], tab_id: u32) -> Vec<ZellijPaneRow> {
    let mut tab_panes: Vec<ZellijPaneRow> =
        panes.iter().filter(|pane| pane.tab_id == tab_id && !pane.is_floating).cloned().collect();
    tab_panes.sort_by_key(|pane| (pane.pane_y, pane.pane_x, pane.id));
    tab_panes
}

fn map_pane_targets(
    target: &ZellijTarget,
    tab: &ZellijTabRow,
    panes: &[ZellijPaneRow],
    pane_targets: &mut HashMap<PaneId, ZellijPaneTarget>,
) -> Vec<PaneId> {
    panes
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
        .collect()
}

fn focused_pane_id(
    target: &ZellijTarget,
    tab: &ZellijTabRow,
    panes: &[ZellijPaneRow],
    pane_ids: &[PaneId],
) -> Option<PaneId> {
    panes
        .iter()
        .find(|pane| pane.is_focused)
        .map(|pane| deterministic_pane_id(target, tab.tab_id, &pane.backend_ref()))
        .or_else(|| pane_ids.first().copied())
}

fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}
