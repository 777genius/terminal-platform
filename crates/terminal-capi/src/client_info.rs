use crate::prelude::*;

#[unsafe(no_mangle)]
pub extern "C" fn terminal_capi_client_binding_version_json(
    client: *mut TerminalCapiClientHandle,
) -> TerminalCapiStringResult {
    match with_client_handle(client, |client| client.client.binding_version()) {
        Ok(version) => TerminalCapiStringResult::ok_json(&version),
        Err(error) => error,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn terminal_capi_client_handshake_info_json(
    client: *mut TerminalCapiClientHandle,
) -> TerminalCapiStringResult {
    match with_client_handle(client, |client| {
        client.runtime.block_on(client.client.handshake_info())
    }) {
        Ok(Ok(handshake)) => TerminalCapiStringResult::ok_json(&handshake),
        Ok(Err(error)) => TerminalCapiStringResult::protocol_error(error),
        Err(error) => error,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn terminal_capi_client_list_sessions_json(
    client: *mut TerminalCapiClientHandle,
) -> TerminalCapiStringResult {
    match with_client_handle(client, |client| {
        client.runtime.block_on(client.client.list_sessions())
    }) {
        Ok(Ok(listed)) => TerminalCapiStringResult::ok_json(&listed),
        Ok(Err(error)) => TerminalCapiStringResult::protocol_error(error),
        Err(error) => error,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn terminal_capi_client_list_saved_sessions_json(
    client: *mut TerminalCapiClientHandle,
) -> TerminalCapiStringResult {
    match with_client_handle(client, |client| {
        client.runtime.block_on(client.client.list_saved_sessions())
    }) {
        Ok(Ok(listed)) => TerminalCapiStringResult::ok_json(&listed),
        Ok(Err(error)) => TerminalCapiStringResult::protocol_error(error),
        Err(error) => error,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn terminal_capi_client_discover_sessions_json(
    client: *mut TerminalCapiClientHandle,
    backend: *const c_char,
) -> TerminalCapiStringResult {
    let backend = match read_backend_kind(backend, "backend") {
        Ok(backend) => backend,
        Err(error) => return error,
    };

    match with_client_handle(client, |client| {
        client.runtime.block_on(client.client.discover_sessions(backend))
    }) {
        Ok(Ok(discovered)) => TerminalCapiStringResult::ok_json(&discovered),
        Ok(Err(error)) => TerminalCapiStringResult::protocol_error(error),
        Err(error) => error,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn terminal_capi_client_backend_capabilities_json(
    client: *mut TerminalCapiClientHandle,
    backend: *const c_char,
) -> TerminalCapiStringResult {
    let backend = match read_backend_kind(backend, "backend") {
        Ok(backend) => backend,
        Err(error) => return error,
    };

    match with_client_handle(client, |client| {
        client.runtime.block_on(client.client.backend_capabilities(backend))
    }) {
        Ok(Ok(capabilities)) => TerminalCapiStringResult::ok_json(&capabilities),
        Ok(Err(error)) => TerminalCapiStringResult::protocol_error(error),
        Err(error) => error,
    }
}
