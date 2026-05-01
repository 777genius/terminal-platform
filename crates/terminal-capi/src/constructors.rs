use crate::prelude::*;

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
