use terminal_backend_api::{
    BackendError, NewTabSpec, OverrideLayoutSpec, ResizePaneSpec, SendInputSpec, SendPasteSpec,
    SplitPaneSpec,
};
use terminal_domain::{DegradedModeReason, PaneId, TabId};
use terminal_mux_domain::SplitDirection;

use super::{
    layout::{reflow_tab_layout, validate_layout_override},
    model::{NativePaneLayoutNode, NativeSessionState, PaneGeometry},
    process::{spawn_pane, spawn_tab},
};

pub(super) fn dispatch_new_tab(
    state: &mut NativeSessionState,
    spec: NewTabSpec,
) -> Result<bool, BackendError> {
    let tab = spawn_tab(spec.title.clone(), &state.launch, state.rows, state.cols)?;
    state.focused_tab = tab.tab_id;
    state.tabs.push(tab);
    Ok(true)
}

pub(super) fn dispatch_split_pane(
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

pub(super) fn dispatch_focus_tab(
    state: &mut NativeSessionState,
    tab_id: TabId,
) -> Result<bool, BackendError> {
    if !state.tabs.iter().any(|tab| tab.tab_id == tab_id) {
        return Err(BackendError::not_found(format!("unknown tab {tab_id:?}")));
    }

    if state.focused_tab == tab_id {
        return Ok(false);
    }

    state.focused_tab = tab_id;
    Ok(true)
}

pub(super) fn dispatch_rename_tab(
    state: &mut NativeSessionState,
    tab_id: TabId,
    title: String,
) -> Result<(bool, Vec<PaneId>), BackendError> {
    let tab = state
        .tabs
        .iter_mut()
        .find(|tab| tab.tab_id == tab_id)
        .ok_or_else(|| BackendError::not_found(format!("unknown tab {tab_id:?}")))?;

    if tab.title.as_deref() == Some(title.as_str()) {
        return Ok((false, Vec::new()));
    }

    tab.title = Some(title);
    Ok((true, tab.pane_ids()))
}

pub(super) fn dispatch_focus_pane(
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

pub(super) fn dispatch_close_tab(
    state: &mut NativeSessionState,
    tab_id: TabId,
) -> Result<bool, BackendError> {
    if state.tabs.len() == 1 {
        return Err(BackendError::invalid_input("native session must keep at least one tab"));
    }

    let index = state
        .tabs
        .iter()
        .position(|tab| tab.tab_id == tab_id)
        .ok_or_else(|| BackendError::not_found(format!("unknown tab {tab_id:?}")))?;
    let removed_focused = state.focused_tab == tab_id;
    state.tabs.remove(index);

    if removed_focused {
        let replacement_index = index.min(state.tabs.len().saturating_sub(1));
        state.focused_tab = state.tabs[replacement_index].tab_id;
    }

    Ok(true)
}

pub(super) fn dispatch_close_pane(
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

pub(super) fn dispatch_resize_pane(
    state: &mut NativeSessionState,
    spec: ResizePaneSpec,
) -> Result<bool, BackendError> {
    let tab = state
        .tabs
        .iter_mut()
        .find(|tab| tab.contains_pane(spec.pane_id))
        .ok_or_else(|| BackendError::not_found(format!("unknown pane {:?}", spec.pane_id)))?;
    let pane = tab.pane(spec.pane_id).ok_or_else(|| {
        BackendError::internal(format!("native tab lost pane {:?}", spec.pane_id))
    })?;
    let current = pane.geometry()?;
    if current.rows == spec.rows && current.cols == spec.cols {
        return Ok(false);
    }

    if tab.panes.len() == 1 {
        pane.resize(spec.rows, spec.cols)?;
        return Ok(true);
    }

    let desired = PaneGeometry { rows: spec.rows.max(1), cols: spec.cols.max(1) };
    if desired.rows != current.rows
        && !tab.root.path_has_direction(spec.pane_id, SplitDirection::Horizontal)
    {
        return Err(BackendError::unsupported(
            "native pane resize cannot independently change rows in current layout",
            DegradedModeReason::ResizeAuthorityExternal,
        ));
    }
    if desired.cols != current.cols
        && !tab.root.path_has_direction(spec.pane_id, SplitDirection::Vertical)
    {
        return Err(BackendError::unsupported(
            "native pane resize cannot independently change cols in current layout",
            DegradedModeReason::ResizeAuthorityExternal,
        ));
    }
    let outcome = tab.root.resize_target(spec.pane_id, desired, state.rows, state.cols);
    if !outcome.changed {
        return Ok(false);
    }

    reflow_tab_layout(tab, state.rows, state.cols)?;
    Ok(true)
}

pub(super) fn dispatch_override_layout(
    state: &mut NativeSessionState,
    spec: OverrideLayoutSpec,
) -> Result<(bool, Vec<PaneId>), BackendError> {
    let tab = state
        .tabs
        .iter_mut()
        .find(|tab| tab.tab_id == spec.tab_id)
        .ok_or_else(|| BackendError::not_found(format!("unknown tab {:?}", spec.tab_id)))?;
    let current_snapshot = tab.root.snapshot();
    if current_snapshot == spec.root {
        return Ok((false, Vec::new()));
    }

    validate_layout_override(tab, &spec.root)?;
    tab.root = NativePaneLayoutNode::from_snapshot(spec.root);
    if !tab.contains_pane(tab.focused_pane) {
        tab.focused_pane = tab
            .first_pane_id()
            .ok_or_else(|| BackendError::internal("native layout override lost all panes"))?;
    }
    reflow_tab_layout(tab, state.rows, state.cols)?;

    Ok((true, tab.pane_ids()))
}

pub(super) fn dispatch_send_input(
    state: &NativeSessionState,
    spec: SendInputSpec,
) -> Result<bool, BackendError> {
    let pane = state
        .tabs
        .iter()
        .find_map(|tab| tab.pane(spec.pane_id))
        .ok_or_else(|| BackendError::not_found(format!("unknown pane {:?}", spec.pane_id)))?;
    pane.write_text(&spec.data)?;
    Ok(false)
}

pub(super) fn dispatch_send_paste(
    state: &NativeSessionState,
    spec: SendPasteSpec,
) -> Result<bool, BackendError> {
    let pane = state
        .tabs
        .iter()
        .find_map(|tab| tab.pane(spec.pane_id))
        .ok_or_else(|| BackendError::not_found(format!("unknown pane {:?}", spec.pane_id)))?;
    pane.write_text(&spec.data)?;
    Ok(false)
}
