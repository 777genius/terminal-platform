use napi::{Error, Result, Status};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use terminal_protocol::ProtocolError;

pub(crate) fn to_json<T>(value: T) -> Result<Value>
where
    T: Serialize,
{
    serde_json::to_value(value).map_err(|error| code_error("serialize_failed", error.to_string()))
}

pub(crate) fn from_json<T>(value: Value, code: &'static str) -> Result<T>
where
    T: DeserializeOwned,
{
    serde_json::from_value(value).map_err(|error| code_error(code, error.to_string()))
}

pub(crate) fn protocol_error(error: ProtocolError) -> Error {
    code_error(&error.code, error.message)
}

fn code_error(code: &str, message: impl Into<String>) -> Error {
    Error::new(Status::GenericFailure, format!("{code}: {}", message.into()))
}

#[cfg(test)]
mod tests {
    use super::{code_error, from_json, to_json};

    #[test]
    fn serializes_structured_payloads_to_json_values() {
        let value = to_json(vec!["alpha", "beta"]).expect("json conversion should succeed");

        assert_eq!(value, serde_json::json!(["alpha", "beta"]));
    }

    #[test]
    fn deserializes_session_requests_from_json_values() {
        let request = from_json::<terminal_node::NodeCreateSessionRequest>(
            serde_json::json!({
                "title": "shell",
                "launch": {
                    "program": "/bin/zsh",
                    "args": ["-i"],
                    "cwd": "/tmp"
                }
            }),
            "invalid_create_session_request",
        )
        .expect("json decoding should succeed");

        assert_eq!(request.title.as_deref(), Some("shell"));
        assert_eq!(request.launch.expect("launch should exist").program, "/bin/zsh".to_string());
    }

    #[test]
    fn deserializes_session_requests_with_null_launch() {
        let request = from_json::<terminal_node::NodeCreateSessionRequest>(
            serde_json::json!({
                "title": "shell",
                "launch": null
            }),
            "invalid_create_session_request",
        )
        .expect("json decoding should succeed");

        assert_eq!(request.title.as_deref(), Some("shell"));
        assert!(request.launch.is_none());
    }

    #[test]
    fn prefixes_protocol_codes_into_napi_errors() {
        let error = code_error("invalid_session_id", "bad session id");

        assert_eq!(error.reason, "invalid_session_id: bad session id");
    }
}
