use terminal_backend_api::{BackendError, SendInputSpec, SendPasteSpec};
use terminal_domain::PaneId;

use super::super::model::{NativePaneRuntime, NativeSessionState};

pub(in crate::engine) fn dispatch_send_input(
    state: &NativeSessionState,
    spec: SendInputSpec,
) -> Result<bool, BackendError> {
    write_to_pane(state, spec.pane_id, &spec.data)
}

pub(in crate::engine) fn dispatch_send_paste(
    state: &NativeSessionState,
    spec: SendPasteSpec,
) -> Result<bool, BackendError> {
    write_paste_to_pane(state, spec.pane_id, &spec.data)
}

fn write_to_pane(
    state: &NativeSessionState,
    pane_id: PaneId,
    data: &str,
) -> Result<bool, BackendError> {
    let pane = input_pane(state, pane_id)?;
    pane.write_text(data)?;
    Ok(false)
}

fn write_paste_to_pane(
    state: &NativeSessionState,
    pane_id: PaneId,
    data: &str,
) -> Result<bool, BackendError> {
    let pane = input_pane(state, pane_id)?;
    pane.write_paste(data)?;
    Ok(false)
}

fn input_pane(
    state: &NativeSessionState,
    pane_id: PaneId,
) -> Result<&NativePaneRuntime, BackendError> {
    state
        .tabs
        .iter()
        .find_map(|tab| tab.pane(pane_id))
        .ok_or_else(|| BackendError::not_found(format!("unknown pane {pane_id:?}")))
}
