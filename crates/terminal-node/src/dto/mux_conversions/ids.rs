use crate::dto::prelude::*;

pub(super) fn parse_pane_id(value: &str) -> Result<PaneId, ProtocolError> {
    parse_uuid(value, "invalid_pane_id", "pane").map(PaneId::from)
}

pub(super) fn parse_tab_id(value: &str) -> Result<TabId, ProtocolError> {
    parse_uuid(value, "invalid_tab_id", "tab").map(TabId::from)
}

fn parse_uuid(value: &str, code: &str, label: &str) -> Result<Uuid, ProtocolError> {
    Uuid::parse_str(value).map_err(|error| {
        ProtocolError::new(code, format!("failed to parse {label} id '{value}' - {error}"))
    })
}
