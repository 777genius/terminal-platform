pub(crate) use std::ffi::{CStr, CString, c_char};

pub(crate) use serde::de::DeserializeOwned;
pub(crate) use terminal_node::{
    NodeBackendKind, NodeCreateSessionRequest, NodeMuxCommand, NodeSessionRoute,
    NodeSubscriptionSpec,
};

pub(crate) use crate::ffi_types::{
    TerminalCapiClientResult, TerminalCapiStringResult, TerminalCapiSubscriptionResult,
};
pub(crate) use crate::handles::{
    TerminalCapiClientHandle, TerminalCapiHandleError, TerminalCapiSubscriptionHandle,
};
pub(crate) use crate::input::{
    read_backend_kind, read_json_or_default, read_optional_string, read_required_json,
    read_required_string, subscription_result_from_open_error, with_client_handle,
    with_subscription_handle,
};
