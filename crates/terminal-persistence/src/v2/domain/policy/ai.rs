use super::super::super::*;

pub(in crate::v2) fn validate_ai_action_kind(
    value: &str,
) -> Result<(), TerminalPersistenceV2Error> {
    const ACTION_KINDS: &[&str] =
        &["send_input", "rerun_command", "export", "share", "delete", "open_link"];
    if !ACTION_KINDS.contains(&value) {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "unknown AI action kind: {value}"
        )));
    }
    Ok(())
}
