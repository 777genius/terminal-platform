use terminal_backend_api::MuxCommand;
use terminal_domain::SessionId;
use terminal_persistence::UiInputEventInput;

use super::super::runtime::SessionRuntime;

pub(super) fn v2_input_capture(
    runtime: &SessionRuntime<'_>,
    session_id: SessionId,
    command: &MuxCommand,
) -> Option<UiInputEventInput> {
    let descriptor = runtime.registry().get(session_id)?;
    match command {
        MuxCommand::SendInput(spec) => Some(UiInputEventInput {
            session_id: session_id.0.to_string(),
            route: descriptor.route,
            title: descriptor.title,
            launch: descriptor.launch,
            pane_id: spec.pane_id.0.to_string(),
            data: spec.data.clone(),
            is_paste: false,
            source_event_id: spec.client_event_id.clone(),
            rows: None,
            cols: None,
            shell_kind: None,
        }),
        MuxCommand::SendPaste(spec) => Some(UiInputEventInput {
            session_id: session_id.0.to_string(),
            route: descriptor.route,
            title: descriptor.title,
            launch: descriptor.launch,
            pane_id: spec.pane_id.0.to_string(),
            data: spec.data.clone(),
            is_paste: true,
            source_event_id: spec.client_event_id.clone(),
            rows: None,
            cols: None,
            shell_kind: None,
        }),
        _ => None,
    }
}
