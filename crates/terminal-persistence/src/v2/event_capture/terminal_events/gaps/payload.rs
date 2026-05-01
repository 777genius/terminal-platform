use super::super::super::super::*;

pub(super) fn history_gap_payload(
    reason: &str,
    skipped_events: u64,
    estimated_dropped_bytes: Option<i64>,
) -> Result<String, TerminalPersistenceV2Error> {
    Ok(serde_json::to_string(&serde_json::json!({
        "reason": reason,
        "skipped_events": skipped_events,
        "estimated_dropped_bytes": estimated_dropped_bytes
    }))?)
}
