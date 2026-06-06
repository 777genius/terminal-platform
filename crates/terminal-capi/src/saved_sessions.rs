use crate::prelude::*;

#[unsafe(no_mangle)]
pub extern "C" fn terminal_capi_client_saved_session_json(
    client: *mut TerminalCapiClientHandle,
    session_id: *const c_char,
) -> TerminalCapiStringResult {
    let session_id = match read_required_string(session_id, "session_id") {
        Ok(session_id) => session_id,
        Err(error) => return error,
    };

    match with_client_handle(client, |client| {
        client.runtime.block_on(client.client.saved_session(&session_id))
    }) {
        Ok(Ok(saved)) => TerminalCapiStringResult::ok_json(&saved),
        Ok(Err(error)) => TerminalCapiStringResult::protocol_error(error),
        Err(error) => error,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn terminal_capi_client_delete_saved_session_json(
    client: *mut TerminalCapiClientHandle,
    session_id: *const c_char,
) -> TerminalCapiStringResult {
    let session_id = match read_required_string(session_id, "session_id") {
        Ok(session_id) => session_id,
        Err(error) => return error,
    };

    match with_client_handle(client, |client| {
        client.runtime.block_on(client.client.delete_saved_session(&session_id))
    }) {
        Ok(Ok(deleted)) => TerminalCapiStringResult::ok_json(&deleted),
        Ok(Err(error)) => TerminalCapiStringResult::protocol_error(error),
        Err(error) => error,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn terminal_capi_client_prune_saved_sessions_json(
    client: *mut TerminalCapiClientHandle,
    keep_latest: usize,
) -> TerminalCapiStringResult {
    match with_client_handle(client, |client| {
        client.runtime.block_on(client.client.prune_saved_sessions(keep_latest))
    }) {
        Ok(Ok(pruned)) => TerminalCapiStringResult::ok_json(&pruned),
        Ok(Err(error)) => TerminalCapiStringResult::protocol_error(error),
        Err(error) => error,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn terminal_capi_client_restore_saved_session_json(
    client: *mut TerminalCapiClientHandle,
    session_id: *const c_char,
) -> TerminalCapiStringResult {
    let session_id = match read_required_string(session_id, "session_id") {
        Ok(session_id) => session_id,
        Err(error) => return error,
    };

    match with_client_handle(client, |client| {
        client.runtime.block_on(client.client.restore_saved_session(&session_id))
    }) {
        Ok(Ok(restored)) => TerminalCapiStringResult::ok_json(&restored),
        Ok(Err(error)) => TerminalCapiStringResult::protocol_error(error),
        Err(error) => error,
    }
}
