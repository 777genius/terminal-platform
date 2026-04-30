use super::super::*;
use super::*;

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

pub(in crate::v2) fn validate_topology_pane_high_water_json_payload(
    topology_snapshot_id: &str,
    pane_high_water_json: &str,
    failures: &mut Vec<String>,
) {
    if let Err(error) = parse_pane_high_water_json(pane_high_water_json) {
        failures.push(format!(
            "topology_snapshot:{topology_snapshot_id} invalid pane_high_water_json: {error}"
        ));
    }
}

pub(in crate::v2) fn parse_pane_high_water_json(
    pane_high_water_json: &str,
) -> Result<BTreeMap<String, i64>, TerminalPersistenceV2Error> {
    let value: Value = serde_json::from_str(pane_high_water_json)?;
    let Some(object) = value.as_object() else {
        return Err(TerminalPersistenceV2Error::InvalidData(
            "pane_high_water_json must be a JSON object".to_string(),
        ));
    };
    let mut high_water = BTreeMap::new();
    for (pane_id, raw_value) in object {
        let Some(value) = raw_value.as_i64() else {
            return Err(TerminalPersistenceV2Error::InvalidData(format!(
                "pane_high_water_json value for pane_id={pane_id} must be an integer"
            )));
        };
        if value < 0 {
            return Err(TerminalPersistenceV2Error::InvalidData(format!(
                "pane_high_water_json value for pane_id={pane_id} must be non-negative"
            )));
        }
        high_water.insert(pane_id.clone(), value);
    }
    Ok(high_water)
}

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
