use std::{ffi::{CString, c_char}, ptr};

use serde::Serialize;
use terminal_protocol::ProtocolError;

use crate::handles::TerminalCapiClientHandle;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalCapiStatus {
    Ok = 0,
    NullPointer = 1,
    InvalidUtf8 = 2,
    InvalidJson = 3,
    ProtocolError = 4,
    RuntimeError = 5,
}

#[repr(C)]
#[derive(Debug)]
pub struct TerminalCapiStringResult {
    pub status: TerminalCapiStatus,
    pub value: *mut c_char,
}

#[repr(C)]
#[derive(Debug)]
pub struct TerminalCapiClientResult {
    pub status: TerminalCapiStatus,
    pub client: *mut TerminalCapiClientHandle,
    pub error: *mut c_char,
}

impl TerminalCapiStringResult {
    #[must_use]
    pub fn ok_json<T>(value: &T) -> Self
    where
        T: Serialize,
    {
        match serde_json::to_string(value) {
            Ok(json) => Self { status: TerminalCapiStatus::Ok, value: into_raw_c_string(json) },
            Err(error) => Self::runtime_error("serialize_failed", error.to_string()),
        }
    }

    #[must_use]
    pub fn null_pointer(name: &str) -> Self {
        Self::runtime_error("null_pointer", format!("{name} must not be null"))
    }

    #[must_use]
    pub fn invalid_utf8(name: &str) -> Self {
        Self::runtime_error("invalid_utf8", format!("{name} must be valid UTF-8"))
    }

    #[must_use]
    pub fn invalid_json(name: &str, error: serde_json::Error) -> Self {
        Self {
            status: TerminalCapiStatus::InvalidJson,
            value: into_raw_c_string(
                serde_json::json!({
                    "code": "invalid_json",
                    "message": format!("{name} did not contain valid JSON: {error}"),
                })
                .to_string(),
            ),
        }
    }

    #[must_use]
    pub fn protocol_error(error: ProtocolError) -> Self {
        match serde_json::to_string(&error) {
            Ok(json) => Self { status: TerminalCapiStatus::ProtocolError, value: into_raw_c_string(json) },
            Err(error) => Self::runtime_error("serialize_failed", error.to_string()),
        }
    }

    #[must_use]
    pub fn runtime_error(code: &str, message: impl Into<String>) -> Self {
        Self {
            status: TerminalCapiStatus::RuntimeError,
            value: into_raw_c_string(
                serde_json::json!({
                    "code": code,
                    "message": message.into(),
                })
                .to_string(),
            ),
        }
    }
}

impl TerminalCapiClientResult {
    #[must_use]
    pub fn ok(client: *mut TerminalCapiClientHandle) -> Self {
        Self { status: TerminalCapiStatus::Ok, client, error: ptr::null_mut() }
    }

    #[must_use]
    pub fn runtime_error(code: &str, message: impl Into<String>) -> Self {
        Self {
            status: TerminalCapiStatus::RuntimeError,
            client: ptr::null_mut(),
            error: into_raw_c_string(
                serde_json::json!({
                    "code": code,
                    "message": message.into(),
                })
                .to_string(),
            ),
        }
    }
}

impl From<TerminalCapiStringResult> for TerminalCapiClientResult {
    fn from(value: TerminalCapiStringResult) -> Self {
        Self { status: value.status, client: ptr::null_mut(), error: value.value }
    }
}

fn into_raw_c_string(value: String) -> *mut c_char {
    let sanitized = value.replace('\0', "\\u0000");
    match CString::new(sanitized) {
        Ok(value) => value.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}
