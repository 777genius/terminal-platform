use super::super::super::*;

pub(super) fn parse_optional_json(
    value: Option<String>,
) -> Result<Option<Value>, TerminalPersistenceV2Error> {
    value.map(|value| serde_json::from_str(&value)).transpose().map_err(Into::into)
}
