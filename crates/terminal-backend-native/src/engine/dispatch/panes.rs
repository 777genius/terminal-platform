use terminal_backend_api::{BackendError, SplitPaneSpec};
use terminal_domain::PaneId;

use super::super::{layout::reflow_tab_layout, model::NativeSessionState, process::spawn_pane};

pub(in crate::engine) fn dispatch_split_pane(
    state: &mut NativeSessionState,
    spec: SplitPaneSpec,
) -> Result<bool, BackendError> {
    let tab = state
        .tabs
        .iter_mut()
        .find(|tab| tab.contains_pane(spec.pane_id))
        .ok_or_else(|| BackendError::not_found(format!("unknown pane {:?}", spec.pane_id)))?;

    let pane = spawn_pane(&state.launch, state.rows, state.cols)?;
    let new_pane_id = pane.pane_id;
    if !tab.root.split_leaf(spec.pane_id, spec.direction, new_pane_id) {
        return Err(BackendError::not_found(format!("unknown pane {:?}", spec.pane_id)));
    }

    tab.focused_pane = new_pane_id;
    tab.panes.push(pane);
    state.focused_tab = tab.tab_id;
    reflow_tab_layout(tab, state.rows, state.cols)?;
    Ok(true)
}

pub(in crate::engine) fn dispatch_focus_pane(
    state: &mut NativeSessionState,
    pane_id: PaneId,
) -> Result<bool, BackendError> {
    let tab = state
        .tabs
        .iter_mut()
        .find(|tab| tab.contains_pane(pane_id))
        .ok_or_else(|| BackendError::not_found(format!("unknown pane {pane_id:?}")))?;

    if state.focused_tab == tab.tab_id && tab.focused_pane == pane_id {
        return Ok(false);
    }

    state.focused_tab = tab.tab_id;
    tab.focused_pane = pane_id;
    Ok(true)
}

pub(in crate::engine) fn dispatch_close_pane(
    state: &mut NativeSessionState,
    pane_id: PaneId,
) -> Result<bool, BackendError> {
    let tab = state
        .tabs
        .iter_mut()
        .find(|tab| tab.contains_pane(pane_id))
        .ok_or_else(|| BackendError::not_found(format!("unknown pane {pane_id:?}")))?;
    if tab.panes.len() <= 1 {
        return Err(BackendError::invalid_input("native tab must keep at least one pane"));
    }

    let Some(new_root) = tab.root.remove_leaf(pane_id) else {
        return Err(BackendError::not_found(format!("unknown pane {pane_id:?}")));
    };
    tab.root = new_root;
    tab.panes.retain(|pane| pane.pane_id != pane_id);
    if tab.focused_pane == pane_id {
        tab.focused_pane = tab
            .first_pane_id()
            .ok_or_else(|| BackendError::internal("native tab root lost all panes"))?;
    }
    reflow_tab_layout(tab, state.rows, state.cols)?;

    Ok(true)
}
