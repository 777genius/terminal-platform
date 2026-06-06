use super::super::super::*;
use super::super::*;
use super::failures::persist_projection_snapshot_failure;

pub(in crate::v2) fn load_latest_valid_screen_snapshot(
    connection: &mut SqliteConnection,
    session_id: &str,
    pane_id: Option<&str>,
    topology_pane_high_water: Option<&BTreeMap<String, i64>>,
    detected_at_ms: i64,
    evidence_ref: &str,
) -> Result<Option<ScreenSnapshotRow>, TerminalPersistenceV2Error> {
    let mut query = terminal_screen_snapshots::table
        .filter(terminal_screen_snapshots::session_id.eq(session_id))
        .into_boxed();
    if let Some(pane_id) = pane_id {
        query = query.filter(terminal_screen_snapshots::pane_id.eq(pane_id));
    }

    let rows = query
        .order((
            terminal_screen_snapshots::high_water_event_seq.desc(),
            terminal_screen_snapshots::created_at_ms.desc(),
        ))
        .limit(MAX_SNAPSHOT_FALLBACK_CANDIDATES)
        .select(ScreenSnapshotRow::as_select())
        .load::<ScreenSnapshotRow>(connection)?;

    for row in rows {
        if let Some(failure) = screen_snapshot_hydration_failure(&row, topology_pane_high_water) {
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

pub(in crate::v2) fn screen_snapshot_hydration_failure(
    row: &ScreenSnapshotRow,
    topology_pane_high_water: Option<&BTreeMap<String, i64>>,
) -> Option<String> {
    let mut failures = Vec::new();
    validate_checksum_text(
        "screen_snapshot",
        &row.id,
        &row.screen_json,
        &row.checksum_algorithm,
        &row.checksum,
        &mut failures,
    );
    validate_screen_snapshot_topology_high_water(row, topology_pane_high_water, &mut failures);
    failures.into_iter().next()
}

pub(in crate::v2) fn validate_screen_snapshot_topology_high_water(
    row: &ScreenSnapshotRow,
    topology_pane_high_water: Option<&BTreeMap<String, i64>>,
    failures: &mut Vec<String>,
) {
    let Some(topology_pane_high_water) = topology_pane_high_water else {
        return;
    };
    if topology_pane_high_water.is_empty() {
        return;
    }
    if row.high_water_byte_seq.is_none() {
        return;
    }
    let Some(max_event_seq) = topology_pane_high_water.get(&row.pane_id) else {
        failures.push(format!(
            "screen_snapshot:{} pane_id={} is not present in topology high-water vector",
            row.id, row.pane_id
        ));
        return;
    };
    if row.high_water_event_seq > *max_event_seq {
        failures.push(format!(
            "screen_snapshot:{} high_water_event_seq={} exceeds topology high_water_event_seq={} for pane_id={}",
            row.id, row.high_water_event_seq, max_event_seq, row.pane_id
        ));
    }
}
