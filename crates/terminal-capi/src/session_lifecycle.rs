use crate::prelude::*;

#[unsafe(no_mangle)]
pub extern "C" fn terminal_capi_client_create_native_session_json(
    client: *mut TerminalCapiClientHandle,
    request_json: *const c_char,
) -> TerminalCapiStringResult {
    let request =
        match read_json_or_default::<NodeCreateSessionRequest>(request_json, "request_json") {
            Ok(request) => request,
            Err(error) => return error,
        };

    match with_client_handle(client, |client| {
        client.runtime.block_on(client.client.create_native_session(&request))
    }) {
        Ok(Ok(created)) => TerminalCapiStringResult::ok_json(&created),
        Ok(Err(error)) => TerminalCapiStringResult::protocol_error(error),
        Err(error) => error,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn terminal_capi_client_import_session_json(
    client: *mut TerminalCapiClientHandle,
    route_json: *const c_char,
    title: *const c_char,
) -> TerminalCapiStringResult {
    let route = match read_required_json::<NodeSessionRoute>(route_json, "route_json") {
        Ok(route) => route,
        Err(error) => return error,
    };
    let title = match read_optional_string(title, "title") {
        Ok(title) => title,
        Err(error) => return error,
    };

    match with_client_handle(client, |client| {
        client.runtime.block_on(client.client.import_session(&route, title))
    }) {
        Ok(Ok(imported)) => TerminalCapiStringResult::ok_json(&imported),
        Ok(Err(error)) => TerminalCapiStringResult::protocol_error(error),
        Err(error) => error,
    }
}
