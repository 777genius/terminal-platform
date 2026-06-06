use super::super::super::*;
use super::super::*;

pub(in crate::v2) fn persist_projection_snapshot_failure(
    connection: &mut SqliteConnection,
    session_id: Option<&str>,
    failure: &str,
    detected_at_ms: i64,
    evidence_ref: &str,
) -> Result<(), TerminalPersistenceV2Error> {
    let validation = HistoryValidation {
        journal_events_checked: 0,
        stream_segments_checked: 0,
        screen_snapshots_checked: if failure.starts_with("screen_snapshot:") { 1 } else { 0 },
        topology_snapshots_checked: if failure.starts_with("topology_snapshot:") { 1 } else { 0 },
        failures: vec![failure.to_string()],
    };
    persist_history_validation_health_records(
        connection,
        session_id,
        &validation,
        detected_at_ms,
        Some(evidence_ref),
    )
}
