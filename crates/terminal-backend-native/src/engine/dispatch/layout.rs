use terminal_backend_api::{BackendError, OverrideLayoutSpec, ResizePaneSpec};
use terminal_domain::{DegradedModeReason, PaneId};
use terminal_mux_domain::SplitDirection;

use super::super::{
    layout::{reflow_tab_layout, validate_layout_override},
    model::{NativePaneLayoutNode, NativeSessionState, PaneGeometry},
};

pub(in crate::engine) fn dispatch_resize_pane(
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
    validate_resize_authority(tab, &spec, desired, current)?;
    let outcome = tab.root.resize_target(spec.pane_id, desired, state.rows, state.cols);
    if !outcome.changed {
        return Ok(false);
    }

    reflow_tab_layout(tab, state.rows, state.cols)?;
    Ok(true)
}

pub(in crate::engine) fn dispatch_override_layout(
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

fn validate_resize_authority(
    tab: &super::super::model::NativeTabRuntime,
    spec: &ResizePaneSpec,
    desired: PaneGeometry,
    current: PaneGeometry,
) -> Result<(), BackendError> {
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

    Ok(())
}
