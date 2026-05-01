use super::super::super::*;
use super::super::*;
use super::{
    failures::persist_projection_snapshot_failure,
    high_water_json::validate_topology_pane_high_water_json_payload,
};

pub(in crate::v2) fn load_latest_valid_topology_snapshot(
    connection: &mut SqliteConnection,
    session_id: &str,
    detected_at_ms: i64,
    evidence_ref: &str,
) -> Result<Option<TopologySnapshotRow>, TerminalPersistenceV2Error> {
    let schema_ids = terminal_payload_schemas::table
        .select(terminal_payload_schemas::id)
        .load::<String>(connection)?;
    let rows = terminal_topology_snapshots::table
        .filter(terminal_topology_snapshots::session_id.eq(session_id))
        .order((
            terminal_topology_snapshots::high_water_commit_seq.desc(),
            terminal_topology_snapshots::created_at_ms.desc(),
        ))
        .limit(MAX_SNAPSHOT_FALLBACK_CANDIDATES)
        .select(TopologySnapshotRow::as_select())
        .load::<TopologySnapshotRow>(connection)?;

    for row in rows {
        if let Some(failure) = topology_snapshot_hydration_failure(&row, &schema_ids) {
            persist_projection_snapshot_failure(
                connection,
                Some(session_id),
                &failure,
                detected_at_ms,
                evidence_ref,
            )?;
            continue;
        }
        return Ok(Some(row));
    }

    Ok(None)
}

pub(in crate::v2) fn topology_snapshot_hydration_failure(
    row: &TopologySnapshotRow,
    schema_ids: &[String],
) -> Option<String> {
    let mut failures = Vec::new();
    validate_payload_schema_ref(
        "topology_snapshot",
        &row.id,
        true,
        row.payload_schema_id.as_deref(),
        schema_ids,
        &mut failures,
    );
    validate_checksum_text(
        "topology_snapshot",
        &row.id,
        &row.topology_json,
        &row.checksum_algorithm,
        &row.checksum,
        &mut failures,
    );
    validate_topology_pane_high_water_json(row, &mut failures);
    failures.into_iter().next()
}

pub(in crate::v2) fn validate_topology_pane_high_water_json(
    row: &TopologySnapshotRow,
    failures: &mut Vec<String>,
) {
    validate_topology_pane_high_water_json_payload(&row.id, &row.pane_high_water_json, failures);
}
