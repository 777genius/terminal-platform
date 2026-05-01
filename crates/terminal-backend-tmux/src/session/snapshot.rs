use super::{TmuxAttachedSession, TmuxSessionSnapshot};
use crate::{
    layout::{fallback_tree, parse_tmux_layout},
    prelude::*,
    rows::{TmuxPaneRow, TmuxPaneTarget, TmuxTabTarget, TmuxWindowRow, non_empty},
    util::{deterministic_pane_id, deterministic_tab_id},
};

impl TmuxAttachedSession {
    pub(super) fn snapshot(&self) -> Result<TmuxSessionSnapshot, BackendError> {
        let windows_output = self.backend.run(
            Some(&self.target),
            &[
                "list-windows",
                "-t",
                &self.target.session_name,
                "-F",
                "#{window_index}\t#{window_id}\t#{window_name}\t#{window_active}\t#{window_layout}",
            ],
        )?;
        let panes_output = self.backend.run(
            Some(&self.target),
            &[
                "list-panes",
                "-a",
                "-t",
                &self.target.session_name,
                "-F",
                "#{window_id}\t#{pane_id}\t#{pane_index}\t#{pane_title}\t#{pane_active}\t#{pane_width}\t#{pane_height}",
            ],
        )?;

        let mut panes_by_window: HashMap<String, Vec<TmuxPaneRow>> = HashMap::new();
        for line in panes_output.lines().filter(|line| !line.trim().is_empty()) {
            let row = TmuxPaneRow::parse(line)?;
            panes_by_window.entry(row.window_id.clone()).or_default().push(row);
        }

        let mut focused_tab = None;
        let mut tabs = Vec::new();
        let mut pane_targets = HashMap::new();
        let mut tab_targets = HashMap::new();
        for line in windows_output.lines().filter(|line| !line.trim().is_empty()) {
            let window = TmuxWindowRow::parse(line)?;
            let mut panes = panes_by_window.remove(&window.window_id).unwrap_or_default();
            panes.sort_by_key(|pane| pane.pane_index);
            let pane_ids: HashMap<u32, PaneId> = panes
                .iter()
                .map(|pane| {
                    (
                        pane.pane_index,
                        deterministic_pane_id(&self.target, &window.window_id, &pane.pane_id),
                    )
                })
                .collect();

            for pane in &panes {
                let canonical_pane_id =
                    deterministic_pane_id(&self.target, &window.window_id, &pane.pane_id);
                pane_targets.insert(
                    canonical_pane_id,
                    TmuxPaneTarget {
                        target: pane.pane_id.clone(),
                        title: non_empty(&pane.pane_title),
                        rows: pane.pane_height,
                        cols: pane.pane_width,
                    },
                );
            }

            let tab_id = deterministic_tab_id(&self.target, &window.window_id);
            tab_targets.insert(tab_id, TmuxTabTarget { target: window.window_id.clone() });
            let focused_pane = panes
                .iter()
                .find(|pane| pane.pane_active)
                .map(|pane| deterministic_pane_id(&self.target, &window.window_id, &pane.pane_id));
            if window.window_active {
                focused_tab = Some(tab_id);
            }
            tabs.push((
                window.window_index,
                TabSnapshot {
                    tab_id,
                    title: non_empty(&window.window_name),
                    root: parse_tmux_layout(&window.window_layout, &pane_ids).unwrap_or_else(
                        || fallback_tree(panes.iter().map(|pane| pane_ids[&pane.pane_index])),
                    ),
                    focused_pane,
                },
            ));
        }
        tabs.sort_by_key(|(window_index, _)| *window_index);

        Ok(TmuxSessionSnapshot {
            topology: TopologySnapshot {
                session_id: self.session_id,
                backend_kind: BackendKind::Tmux,
                tabs: tabs.into_iter().map(|(_, tab)| tab).collect(),
                focused_tab,
            },
            pane_targets,
            tab_targets,
        })
    }
}
