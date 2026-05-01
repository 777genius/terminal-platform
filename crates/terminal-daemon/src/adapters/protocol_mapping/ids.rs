use terminal_domain::{PaneId, SessionId};
use terminal_protocol::ProtocolError;
use uuid::Uuid;

pub(super) fn parse_session_id(value: &str) -> Result<SessionId, ProtocolError> {
    Uuid::parse_str(value)
        .map(SessionId::from)
        .map_err(|error| ProtocolError::new("invalid_persisted_session_id", error.to_string()))
}

pub(super) fn parse_pane_id(value: &str) -> Result<PaneId, ProtocolError> {
    Uuid::parse_str(value)
        .map(PaneId::from)
        .map_err(|error| ProtocolError::new("invalid_persisted_pane_id", error.to_string()))
}
