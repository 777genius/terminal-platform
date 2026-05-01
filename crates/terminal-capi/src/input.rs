use crate::prelude::*;

pub(crate) fn with_client_handle<T>(
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

pub(crate) fn with_subscription_handle<T>(
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

pub(crate) fn read_required_string(
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

pub(crate) fn read_json_or_default<T>(
    value: *const c_char,
    name: &str,
) -> Result<T, TerminalCapiStringResult>
where
    T: DeserializeOwned + Default,
{
    if value.is_null() {
        return Ok(T::default());
    }

    let json = read_required_string(value, name)?;
    serde_json::from_str(&json).map_err(|error| TerminalCapiStringResult::invalid_json(name, error))
}

pub(crate) fn read_required_json<T>(
    value: *const c_char,
    name: &str,
) -> Result<T, TerminalCapiStringResult>
where
    T: DeserializeOwned,
{
    let json = read_required_string(value, name)?;
    serde_json::from_str(&json).map_err(|error| TerminalCapiStringResult::invalid_json(name, error))
}

pub(crate) fn read_optional_string(
    value: *const c_char,
    name: &str,
) -> Result<Option<String>, TerminalCapiStringResult> {
    if value.is_null() {
        return Ok(None);
    }

    read_required_string(value, name).map(Some)
}

pub(crate) fn read_backend_kind(
    value: *const c_char,
    name: &str,
) -> Result<NodeBackendKind, TerminalCapiStringResult> {
    let value = read_required_string(value, name)?;
    serde_json::from_value(serde_json::Value::String(value))
        .map_err(|error| TerminalCapiStringResult::invalid_json(name, error))
}

pub(crate) fn subscription_result_from_open_error(
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
