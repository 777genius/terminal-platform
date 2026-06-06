use crate::prelude::*;

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
