mod ffi_types;
mod handles;

use std::ffi::{CStr, CString, c_char};

use serde::de::DeserializeOwned;
use terminal_node::{
    NodeBackendKind, NodeCreateSessionRequest, NodeMuxCommand, NodeSessionRoute,
    NodeSubscriptionSpec,
};

pub use ffi_types::{
    TerminalCapiClientResult, TerminalCapiStatus, TerminalCapiStringResult,
    TerminalCapiSubscriptionResult,
};
use handles::{TerminalCapiClientHandle, TerminalCapiHandleError, TerminalCapiSubscriptionHandle};

fn with_client_handle<T>(
    handle: *mut TerminalCapiClientHandle,
    op: impl FnOnce(&mut TerminalCapiClientHandle) -> T,
) -> Result<T, TerminalCapiStringResult> {
    if handle.is_null() {
        return Err(TerminalCapiStringResult::null_pointer("client"));
    }

    let handle = {
        // SAFETY: the null case is handled above, and callers must pass a live handle pointer
        // produced by this crate's constructor functions.
        unsafe { &mut *handle }
    };

    Ok(op(handle))
}

fn with_subscription_handle<T>(
    handle: *mut TerminalCapiSubscriptionHandle,
    op: impl FnOnce(&mut TerminalCapiSubscriptionHandle) -> T,
) -> Result<T, TerminalCapiStringResult> {
    if handle.is_null() {
        return Err(TerminalCapiStringResult::null_pointer("subscription"));
    }

    let handle = {
        // SAFETY: the null case is handled above, and callers must pass a live handle pointer
        // produced by this crate's constructor functions.
        unsafe { &mut *handle }
    };

    Ok(op(handle))
}

fn read_required_string(
    value: *const c_char,
    name: &str,
) -> Result<String, TerminalCapiStringResult> {
    if value.is_null() {
        return Err(TerminalCapiStringResult::null_pointer(name));
    }

    let value = {
        // SAFETY: the pointer is checked for null above and is expected to point to a valid
        // NUL-terminated C string for the duration of this call.
        unsafe { CStr::from_ptr(value) }
    };

    value.to_str().map(str::to_owned).map_err(|_| TerminalCapiStringResult::invalid_utf8(name))
}

fn read_json_or_default<T>(value: *const c_char, name: &str) -> Result<T, TerminalCapiStringResult>
where
    T: DeserializeOwned + Default,
{
    if value.is_null() {
        return Ok(T::default());
    }

    let json = read_required_string(value, name)?;
    serde_json::from_str(&json).map_err(|error| TerminalCapiStringResult::invalid_json(name, error))
}

fn read_required_json<T>(value: *const c_char, name: &str) -> Result<T, TerminalCapiStringResult>
where
    T: DeserializeOwned,
{
    let json = read_required_string(value, name)?;
    serde_json::from_str(&json).map_err(|error| TerminalCapiStringResult::invalid_json(name, error))
}

fn read_optional_string(
    value: *const c_char,
    name: &str,
) -> Result<Option<String>, TerminalCapiStringResult> {
    if value.is_null() {
        return Ok(None);
    }

    read_required_string(value, name).map(Some)
}

fn read_backend_kind(
    value: *const c_char,
    name: &str,
) -> Result<NodeBackendKind, TerminalCapiStringResult> {
    let value = read_required_string(value, name)?;
    serde_json::from_value(serde_json::Value::String(value))
        .map_err(|error| TerminalCapiStringResult::invalid_json(name, error))
}

fn subscription_result_from_open_error(
    error: TerminalCapiHandleError,
) -> TerminalCapiSubscriptionResult {
    match error {
        TerminalCapiHandleError::Runtime(error) => {
            TerminalCapiSubscriptionResult::runtime_error("runtime_init_failed", error.to_string())
        }
        TerminalCapiHandleError::Protocol(error) => {
            TerminalCapiStringResult::protocol_error(error).into()
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn terminal_capi_client_new_from_runtime_slug(
    slug: *const c_char,
) -> TerminalCapiClientResult {
    let slug = match read_required_string(slug, "slug") {
        Ok(slug) => slug,
        Err(error) => return error.into(),
    };

    match TerminalCapiClientHandle::from_runtime_slug(slug) {
        Ok(handle) => TerminalCapiClientResult::ok(Box::into_raw(Box::new(handle))),
        Err(error) => {
            TerminalCapiClientResult::runtime_error("runtime_init_failed", error.to_string())
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn terminal_capi_client_new_from_namespaced_address(
    value: *const c_char,
) -> TerminalCapiClientResult {
    let value = match read_required_string(value, "value") {
        Ok(value) => value,
        Err(error) => return error.into(),
    };

    match TerminalCapiClientHandle::from_namespaced_address(value) {
        Ok(handle) => TerminalCapiClientResult::ok(Box::into_raw(Box::new(handle))),
        Err(error) => {
            TerminalCapiClientResult::runtime_error("runtime_init_failed", error.to_string())
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn terminal_capi_client_new_from_filesystem_path(
    path: *const c_char,
) -> TerminalCapiClientResult {
    let path = match read_required_string(path, "path") {
        Ok(path) => path,
        Err(error) => return error.into(),
    };

    match TerminalCapiClientHandle::from_filesystem_path(path) {
        Ok(handle) => TerminalCapiClientResult::ok(Box::into_raw(Box::new(handle))),
        Err(error) => {
            TerminalCapiClientResult::runtime_error("runtime_init_failed", error.to_string())
        }
    }
}

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

#[unsafe(no_mangle)]
pub extern "C" fn terminal_capi_client_open_subscription(
    client: *mut TerminalCapiClientHandle,
    session_id: *const c_char,
    spec_json: *const c_char,
) -> TerminalCapiSubscriptionResult {
    let session_id = match read_required_string(session_id, "session_id") {
        Ok(session_id) => session_id,
        Err(error) => return error.into(),
    };
    let spec = match read_required_json::<NodeSubscriptionSpec>(spec_json, "spec_json") {
        Ok(spec) => spec,
        Err(error) => return error.into(),
    };

    match with_client_handle(client, |client| {
        TerminalCapiSubscriptionHandle::open(client.client.clone(), session_id, spec)
    }) {
        Ok(Ok(subscription)) => {
            TerminalCapiSubscriptionResult::ok(Box::into_raw(Box::new(subscription)))
        }
        Ok(Err(error)) => subscription_result_from_open_error(error),
        Err(error) => error.into(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn terminal_capi_subscription_meta_json(
    subscription: *mut TerminalCapiSubscriptionHandle,
) -> TerminalCapiStringResult {
    match with_subscription_handle(subscription, |subscription| subscription.subscription.meta()) {
        Ok(meta) => TerminalCapiStringResult::ok_json(&meta),
        Err(error) => error,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn terminal_capi_subscription_next_event_json(
    subscription: *mut TerminalCapiSubscriptionHandle,
) -> TerminalCapiStringResult {
    match with_subscription_handle(subscription, |subscription| {
        subscription.runtime.block_on(subscription.subscription.next_event())
    }) {
        Ok(Ok(event)) => TerminalCapiStringResult::ok_json(&event),
        Ok(Err(error)) => TerminalCapiStringResult::protocol_error(error),
        Err(error) => error,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn terminal_capi_subscription_close(
    subscription: *mut TerminalCapiSubscriptionHandle,
) -> TerminalCapiStringResult {
    match with_subscription_handle(subscription, |subscription| {
        subscription.runtime.block_on(subscription.subscription.close());
    }) {
        Ok(()) => TerminalCapiStringResult::ok_json(&serde_json::json!({ "closed": true })),
        Err(error) => error,
    }
}

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

/// # Safety
///
/// `client` must be a pointer previously returned by one of this crate's
/// `terminal_capi_client_new_*` constructors and must not have been freed yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn terminal_capi_client_free(client: *mut TerminalCapiClientHandle) {
    if client.is_null() {
        return;
    }

    // SAFETY: callers must only free handles previously returned by this crate and not freed yet.
    unsafe {
        drop(Box::from_raw(client));
    }
}

/// # Safety
///
/// `subscription` must be a pointer previously returned by this crate and must
/// not have been freed yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn terminal_capi_subscription_free(
    subscription: *mut TerminalCapiSubscriptionHandle,
) {
    if subscription.is_null() {
        return;
    }

    // SAFETY: callers must only free handles previously returned by this crate and not freed yet.
    unsafe {
        drop(Box::from_raw(subscription));
    }
}

/// # Safety
///
/// `value` must be a pointer previously returned by this crate and must not have
/// been freed yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn terminal_capi_string_free(value: *mut c_char) {
    if value.is_null() {
        return;
    }

    // SAFETY: callers must only free strings previously returned by this crate and not freed yet.
    unsafe {
        drop(CString::from_raw(value));
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::CString;

    use serde_json::Value;
    use terminal_protocol::LocalSocketAddress;
    use terminal_testing::daemon_fixture;

    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn roundtrips_c_api_host_flow_against_daemon_fixture() {
        let fixture = match daemon_fixture("terminal-capi-smoke") {
            Ok(fixture) => fixture,
            Err(error) => panic!("fixture should start: {error}"),
        };
        let address = fixture.client.address().clone();

        let join = std::thread::spawn(move || {
            let handle = match address {
                LocalSocketAddress::Namespaced(value) => {
                    let value = c_string(&value);
                    let result = terminal_capi_client_new_from_namespaced_address(value.as_ptr());
                    assert_eq!(result.status, TerminalCapiStatus::Ok);
                    result.client
                }
                LocalSocketAddress::Filesystem(path) => {
                    let path = c_string(&path.display().to_string());
                    let result = terminal_capi_client_new_from_filesystem_path(path.as_ptr());
                    assert_eq!(result.status, TerminalCapiStatus::Ok);
                    result.client
                }
            };

            let binding_version =
                read_json_result(terminal_capi_client_binding_version_json(handle));
            assert_eq!(binding_version["protocol"]["major"], 0);
            assert_eq!(binding_version["protocol"]["minor"], 1);

            let handshake = read_json_result(terminal_capi_client_handshake_info_json(handle));
            assert_eq!(handshake["assessment"]["can_use"], true);
            let backend = c_string("native");
            let capabilities = read_json_result(terminal_capi_client_backend_capabilities_json(
                handle,
                backend.as_ptr(),
            ));
            assert_eq!(capabilities["backend"], "native");
            assert_eq!(capabilities["capabilities"]["explicit_session_save"], true);

            let create_request = c_string(
                r#"{
                  "title":"capi-smoke",
                  "launch":{
                    "program":"/bin/sh",
                    "args":["-lc","printf 'ready\n'; exec cat"]
                  }
                }"#,
            );
            let created = read_json_result(terminal_capi_client_create_native_session_json(
                handle,
                create_request.as_ptr(),
            ));
            let session_id = created["session_id"].as_str().unwrap_or_default().to_string();
            assert!(!session_id.is_empty());

            let listed = read_json_result(terminal_capi_client_list_sessions_json(handle));
            assert!(
                listed
                    .as_array()
                    .map(|sessions| {
                        sessions.iter().any(|session| {
                            session["session_id"].as_str() == Some(session_id.as_str())
                        })
                    })
                    .unwrap_or(false),
            );

            let session_id_c = c_string(&session_id);
            let attached = read_json_result(terminal_capi_client_attach_session_json(
                handle,
                session_id_c.as_ptr(),
            ));
            let pane_id =
                attached["focused_screen"]["pane_id"].as_str().unwrap_or_default().to_string();
            assert!(!pane_id.is_empty());

            let topology_subscription_spec = c_string(r#"{"kind":"session_topology"}"#);
            let topology_subscription =
                read_subscription_result(terminal_capi_client_open_subscription(
                    handle,
                    session_id_c.as_ptr(),
                    topology_subscription_spec.as_ptr(),
                ));
            let topology_meta =
                read_json_result(terminal_capi_subscription_meta_json(topology_subscription));
            assert!(!topology_meta["subscription_id"].as_str().unwrap_or_default().is_empty());
            let initial_topology_event =
                read_json_result(terminal_capi_subscription_next_event_json(topology_subscription));
            assert_eq!(initial_topology_event["kind"], "topology_snapshot");
            assert_eq!(initial_topology_event["tabs"].as_array().map_or(0, Vec::len), 1);

            let pane_subscription_spec =
                c_string(&format!(r#"{{"kind":"pane_surface","pane_id":"{pane_id}"}}"#));
            let pane_subscription =
                read_subscription_result(terminal_capi_client_open_subscription(
                    handle,
                    session_id_c.as_ptr(),
                    pane_subscription_spec.as_ptr(),
                ));
            let pane_meta =
                read_json_result(terminal_capi_subscription_meta_json(pane_subscription));
            assert!(!pane_meta["subscription_id"].as_str().unwrap_or_default().is_empty());
            let initial_pane_event =
                read_json_result(terminal_capi_subscription_next_event_json(pane_subscription));
            assert_eq!(initial_pane_event["kind"], "screen_delta");
            assert!(initial_pane_event["full_replace"].is_object());

            let pane_id_c = c_string(&pane_id);
            let snapshot = read_json_result(terminal_capi_client_screen_snapshot_json(
                handle,
                session_id_c.as_ptr(),
                pane_id_c.as_ptr(),
            ));
            assert_eq!(snapshot["pane_id"], pane_id);

            let delta = read_json_result(terminal_capi_client_screen_delta_json(
                handle,
                session_id_c.as_ptr(),
                pane_id_c.as_ptr(),
                0,
            ));
            assert_eq!(delta["pane_id"], pane_id);

            let mux_command = c_string(r#"{"kind":"new_tab","title":"ffi"}"#);
            let dispatch = read_json_result(terminal_capi_client_dispatch_mux_command_json(
                handle,
                session_id_c.as_ptr(),
                mux_command.as_ptr(),
            ));
            assert_eq!(dispatch["changed"], true);
            let topology_update = wait_for_topology_event(topology_subscription, 2);
            assert_eq!(topology_update["tabs"].as_array().map_or(0, Vec::len), 2);

            let input_command = c_string(&format!(
                r#"{{"kind":"send_input","pane_id":"{pane_id}","data":"ffi subscription input\r"}}"#
            ));
            let input_result = read_json_result(terminal_capi_client_dispatch_mux_command_json(
                handle,
                session_id_c.as_ptr(),
                input_command.as_ptr(),
            ));
            assert_eq!(input_result["changed"], false);
            let pane_update =
                wait_for_screen_delta_with_text(pane_subscription, "ffi subscription input");
            assert_eq!(pane_update["kind"], "screen_delta");

            let save_command = c_string(r#"{"kind":"save_session"}"#);
            let save = read_json_result(terminal_capi_client_dispatch_mux_command_json(
                handle,
                session_id_c.as_ptr(),
                save_command.as_ptr(),
            ));
            assert_eq!(save["changed"], false);

            let saved_sessions =
                read_json_result(terminal_capi_client_list_saved_sessions_json(handle));
            assert!(
                saved_sessions
                    .as_array()
                    .map(|sessions| {
                        sessions.iter().any(|session| {
                            session["session_id"].as_str() == Some(session_id.as_str())
                        })
                    })
                    .unwrap_or(false),
            );

            let saved = read_json_result(terminal_capi_client_saved_session_json(
                handle,
                session_id_c.as_ptr(),
            ));
            assert_eq!(saved["session_id"], session_id);
            assert_eq!(saved["compatibility"]["can_restore"], true);

            let restored = read_json_result(terminal_capi_client_restore_saved_session_json(
                handle,
                session_id_c.as_ptr(),
            ));
            assert_eq!(restored["saved_session_id"], session_id);

            let topology = read_json_result(terminal_capi_client_topology_snapshot_json(
                handle,
                session_id_c.as_ptr(),
            ));
            assert_eq!(topology["tabs"].as_array().map_or(0, Vec::len), 2);

            let deleted = read_json_result(terminal_capi_client_delete_saved_session_json(
                handle,
                session_id_c.as_ptr(),
            ));
            assert_eq!(deleted["session_id"], session_id);

            let closed_topology =
                read_json_result(terminal_capi_subscription_close(topology_subscription));
            assert_eq!(closed_topology["closed"], true);

            let closed_pane = read_json_result(terminal_capi_subscription_close(pane_subscription));
            assert_eq!(closed_pane["closed"], true);

            // SAFETY: subscription handles were returned by this crate and are freed exactly once.
            unsafe {
                terminal_capi_subscription_free(topology_subscription);
                terminal_capi_subscription_free(pane_subscription);
            }

            // SAFETY: handle was returned by this crate and is freed exactly once here.
            unsafe { terminal_capi_client_free(handle) };
        });

        match join.join() {
            Ok(()) => {}
            Err(_) => panic!("c api worker thread should succeed"),
        }

        fixture
            .shutdown()
            .await
            .unwrap_or_else(|error| panic!("fixture should stop cleanly: {error}"));
    }

    fn c_string(value: &str) -> CString {
        match CString::new(value) {
            Ok(value) => value,
            Err(error) => panic!("CString creation should succeed: {error}"),
        }
    }

    fn read_json_result(result: TerminalCapiStringResult) -> Value {
        assert_eq!(result.status, TerminalCapiStatus::Ok);
        let raw = result.value;
        assert!(!raw.is_null());

        let owned = {
            // SAFETY: the pointer comes from this crate and is consumed exactly once here.
            unsafe { CString::from_raw(raw) }
        };
        let value = match owned.into_string() {
            Ok(value) => value,
            Err(error) => error.into_cstring().to_string_lossy().into_owned(),
        };

        serde_json::from_str(&value)
            .unwrap_or_else(|error| panic!("result JSON should parse: {error}\n{value}"))
    }

    fn read_subscription_result(
        result: TerminalCapiSubscriptionResult,
    ) -> *mut TerminalCapiSubscriptionHandle {
        assert_eq!(result.status, TerminalCapiStatus::Ok);
        assert!(!result.subscription.is_null());
        result.subscription
    }

    fn wait_for_topology_event(
        subscription: *mut TerminalCapiSubscriptionHandle,
        expected_tabs: usize,
    ) -> Value {
        for _ in 0..8 {
            let event = read_json_result(terminal_capi_subscription_next_event_json(subscription));
            if event["kind"] == "topology_snapshot"
                && event["tabs"].as_array().map_or(0, Vec::len) == expected_tabs
            {
                return event;
            }
        }

        panic!("topology subscription should yield snapshot with {expected_tabs} tabs");
    }

    fn wait_for_screen_delta_with_text(
        subscription: *mut TerminalCapiSubscriptionHandle,
        needle: &str,
    ) -> Value {
        for _ in 0..16 {
            let event = read_json_result(terminal_capi_subscription_next_event_json(subscription));
            if event["kind"] == "screen_delta" && json_value_contains_text(&event, needle) {
                return event;
            }
        }

        panic!("pane subscription should yield screen delta containing `{needle}`");
    }

    fn json_value_contains_text(value: &Value, needle: &str) -> bool {
        match value {
            Value::Null | Value::Bool(_) | Value::Number(_) => false,
            Value::String(text) => text.contains(needle),
            Value::Array(items) => items.iter().any(|item| json_value_contains_text(item, needle)),
            Value::Object(entries) => {
                entries.values().any(|item| json_value_contains_text(item, needle))
            }
        }
    }
}
