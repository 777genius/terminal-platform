use crate::prelude::*;

#[unsafe(no_mangle)]
pub extern "C" fn terminal_capi_client_attach_session_json(
    client: *mut TerminalCapiClientHandle,
    session_id: *const c_char,
) -> TerminalCapiStringResult {
    let session_id = match read_required_string(session_id, "session_id") {
        Ok(session_id) => session_id,
        Err(error) => return error,
    };

    match with_client_handle(client, |client| {
        client.runtime.block_on(client.client.attach_session(&session_id))
    }) {
        Ok(Ok(attached)) => TerminalCapiStringResult::ok_json(&attached),
        Ok(Err(error)) => TerminalCapiStringResult::protocol_error(error),
        Err(error) => error,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn terminal_capi_client_topology_snapshot_json(
    client: *mut TerminalCapiClientHandle,
    session_id: *const c_char,
) -> TerminalCapiStringResult {
    let session_id = match read_required_string(session_id, "session_id") {
        Ok(session_id) => session_id,
        Err(error) => return error,
    };

    match with_client_handle(client, |client| {
        client.runtime.block_on(client.client.topology_snapshot(&session_id))
    }) {
        Ok(Ok(snapshot)) => TerminalCapiStringResult::ok_json(&snapshot),
        Ok(Err(error)) => TerminalCapiStringResult::protocol_error(error),
        Err(error) => error,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn terminal_capi_client_session_health_snapshot_json(
    client: *mut TerminalCapiClientHandle,
    session_id: *const c_char,
) -> TerminalCapiStringResult {
    let session_id = match read_required_string(session_id, "session_id") {
        Ok(session_id) => session_id,
        Err(error) => return error,
    };

    match with_client_handle(client, |client| {
        client.runtime.block_on(client.client.session_health_snapshot(&session_id))
    }) {
        Ok(Ok(snapshot)) => TerminalCapiStringResult::ok_json(&snapshot),
        Ok(Err(error)) => TerminalCapiStringResult::protocol_error(error),
        Err(error) => error,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn terminal_capi_client_screen_snapshot_json(
    client: *mut TerminalCapiClientHandle,
    session_id: *const c_char,
    pane_id: *const c_char,
) -> TerminalCapiStringResult {
    let session_id = match read_required_string(session_id, "session_id") {
        Ok(session_id) => session_id,
        Err(error) => return error,
    };
    let pane_id = match read_required_string(pane_id, "pane_id") {
        Ok(pane_id) => pane_id,
        Err(error) => return error,
    };

    match with_client_handle(client, |client| {
        client.runtime.block_on(client.client.screen_snapshot(&session_id, &pane_id))
    }) {
        Ok(Ok(snapshot)) => TerminalCapiStringResult::ok_json(&snapshot),
        Ok(Err(error)) => TerminalCapiStringResult::protocol_error(error),
        Err(error) => error,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn terminal_capi_client_screen_delta_json(
    client: *mut TerminalCapiClientHandle,
    session_id: *const c_char,
    pane_id: *const c_char,
    from_sequence: u64,
) -> TerminalCapiStringResult {
    let session_id = match read_required_string(session_id, "session_id") {
        Ok(session_id) => session_id,
        Err(error) => return error,
    };
    let pane_id = match read_required_string(pane_id, "pane_id") {
        Ok(pane_id) => pane_id,
        Err(error) => return error,
    };

    match with_client_handle(client, |client| {
        client.runtime.block_on(client.client.screen_delta(&session_id, &pane_id, from_sequence))
    }) {
        Ok(Ok(delta)) => TerminalCapiStringResult::ok_json(&delta),
        Ok(Err(error)) => TerminalCapiStringResult::protocol_error(error),
        Err(error) => error,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn terminal_capi_client_dispatch_mux_command_json(
    client: *mut TerminalCapiClientHandle,
    session_id: *const c_char,
    command_json: *const c_char,
) -> TerminalCapiStringResult {
    let session_id = match read_required_string(session_id, "session_id") {
        Ok(session_id) => session_id,
        Err(error) => return error,
    };
    let command = match read_required_json::<NodeMuxCommand>(command_json, "command_json") {
        Ok(command) => command,
        Err(error) => return error,
    };

    match with_client_handle(client, |client| {
        client.runtime.block_on(client.client.dispatch_mux_command(&session_id, &command))
    }) {
        Ok(Ok(result)) => TerminalCapiStringResult::ok_json(&result),
        Ok(Err(error)) => TerminalCapiStringResult::protocol_error(error),
        Err(error) => error,
    }
}
