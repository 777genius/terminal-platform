use crate::prelude::*;

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
