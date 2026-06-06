use terminal_backend_api::BackendError;
use terminal_domain::PaneId;

use super::super::model::{NativePaneLayoutNode, NativeSessionState, NativeTabRuntime};

pub(in crate::engine) fn reflow_tab_layout(
    tab: &NativeTabRuntime,
    rows: u16,
    cols: u16,
) -> Result<(), BackendError> {
    apply_pane_layout(&tab.root, tab, rows.max(1), cols.max(1))
}

pub(in crate::engine) fn collect_surface_updates(
    state: &NativeSessionState,
    pane_id: PaneId,
) -> Vec<PaneId> {
    state
        .tabs
        .iter()
        .find(|tab| tab.contains_pane(pane_id))
        .map_or_else(Vec::new, NativeTabRuntime::pane_ids)
}

fn apply_pane_layout(
    node: &NativePaneLayoutNode,
    tab: &NativeTabRuntime,
    rows: u16,
    cols: u16,
) -> Result<(), BackendError> {
    match node {
        NativePaneLayoutNode::Leaf { pane_id } => {
            let pane = tab.pane(*pane_id).ok_or_else(|| {
                BackendError::internal(format!(
                    "native pane tree references missing pane {pane_id:?}"
                ))
            })?;
            pane.resize(rows, cols)?;
            Ok(())
        }
        NativePaneLayoutNode::Split(split) => {
            let ((first_rows, first_cols), (second_rows, second_cols)) =
                split.partition(rows, cols);
            apply_pane_layout(&split.first, tab, first_rows, first_cols)?;
            apply_pane_layout(&split.second, tab, second_rows, second_cols)
        }
    }
}
