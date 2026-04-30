use super::*;

pub(super) fn verify_seeded_defaults(
    connection: &mut SqliteConnection,
) -> Result<(), TerminalPersistenceV2Error> {
    let identity = terminal_db_identity::table
        .select(DbIdentityProbeRow::as_select())
        .first::<DbIdentityProbeRow>(connection)
        .optional()?;
    if identity.is_none() {
        return Err(TerminalPersistenceV2Error::InvalidData(
            "terminal_db_identity was not initialized".to_string(),
        ));
    }

    let gate_count: i64 = terminal_feature_gates::table.count().get_result(connection)?;
    if gate_count == 0 {
        return Err(TerminalPersistenceV2Error::InvalidData(
            "terminal_feature_gates seed rows are missing".to_string(),
        ));
    }

    seed_payload_schemas(connection, current_time_ms())?;
    let schema_count: i64 = terminal_payload_schemas::table.count().get_result(connection)?;
    if schema_count == 0 {
        return Err(TerminalPersistenceV2Error::InvalidData(
            "terminal_payload_schemas seed rows are missing".to_string(),
        ));
    }

    Ok(())
}

pub(super) fn load_feature_gate_state(
    connection: &mut SqliteConnection,
    name: FeatureGateName,
) -> Result<FeatureGateState, TerminalPersistenceV2Error> {
    let row = terminal_feature_gates::table
        .filter(terminal_feature_gates::feature_name.eq(name.as_str()))
        .select(FeatureGateRow::as_select())
        .first::<FeatureGateRow>(connection)?;
    FeatureGateState::parse(&row.state)
}

pub(super) fn validate_feature_gate_transition(
    connection: &mut SqliteConnection,
    config: &TerminalPersistenceV2Config,
    name: FeatureGateName,
    state: FeatureGateState,
) -> Result<(), TerminalPersistenceV2Error> {
    if name == FeatureGateName::TerminalPersistenceV2AuthoritativeReads
        && state == FeatureGateState::Enabled
        && load_feature_gate_state(connection, FeatureGateName::TerminalPersistenceV2Authoritative)?
            != FeatureGateState::Enabled
    {
        return Err(TerminalPersistenceV2Error::InvalidData(
            "terminal_persistence_v2_authoritative_reads requires terminal_persistence_v2_authoritative=enabled".to_string(),
        ));
    }

    if name == FeatureGateName::TerminalPersistenceV2Authoritative
        && matches!(state, FeatureGateState::Disabled | FeatureGateState::ForceDisabled)
        && load_feature_gate_state(
            connection,
            FeatureGateName::TerminalPersistenceV2AuthoritativeReads,
        )? == FeatureGateState::Enabled
    {
        return Err(TerminalPersistenceV2Error::InvalidData(
            "disable terminal_persistence_v2_authoritative_reads first before disabling terminal_persistence_v2_authoritative".to_string(),
        ));
    }

    if name == FeatureGateName::EncryptedTerminalHistory
        && state == FeatureGateState::Enabled
        && !encryption_capability_state_for_connection(connection, config)?
            .can_enable_encrypted_history
    {
        return Err(TerminalPersistenceV2Error::InvalidData(
            "encrypted_terminal_history requires an active non-test database key".to_string(),
        ));
    }

    Ok(())
}

pub(super) fn enforce_encryption_startup_policy(
    connection: &mut SqliteConnection,
    config: &TerminalPersistenceV2Config,
) -> Result<(), TerminalPersistenceV2Error> {
    let capability = encryption_capability_state_for_connection(connection, config)?;
    if capability.feature_gate_state == FeatureGateState::Enabled.as_str()
        && !capability.can_enable_encrypted_history
    {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "encrypted terminal history cannot start: {}",
            capability.action_required
        )));
    }
    Ok(())
}

pub(super) fn encryption_capability_state_for_connection(
    connection: &mut SqliteConnection,
    config: &TerminalPersistenceV2Config,
) -> Result<EncryptionCapabilityRecord, TerminalPersistenceV2Error> {
    let feature_gate_state =
        load_feature_gate_state(connection, FeatureGateName::EncryptedTerminalHistory)?
            .as_str()
            .to_string();
    let active_database_key_count = terminal_crypto_keys::table
        .filter(terminal_crypto_keys::key_kind.eq("database_key"))
        .filter(terminal_crypto_keys::state.eq("active"))
        .count()
        .get_result::<i64>(connection)?;
    let active_non_test_database_key_count = terminal_crypto_keys::table
        .filter(terminal_crypto_keys::key_kind.eq("database_key"))
        .filter(terminal_crypto_keys::state.eq("active"))
        .filter(terminal_crypto_keys::protection_kind.ne("test_plaintext"))
        .count()
        .get_result::<i64>(connection)?;
    let test_plaintext_database_key_count = terminal_crypto_keys::table
        .filter(terminal_crypto_keys::key_kind.eq("database_key"))
        .filter(terminal_crypto_keys::state.eq("active"))
        .filter(terminal_crypto_keys::protection_kind.eq("test_plaintext"))
        .count()
        .get_result::<i64>(connection)?;
    let unavailable_key_count = terminal_crypto_keys::table
        .filter(terminal_crypto_keys::state.eq("unavailable"))
        .count()
        .get_result::<i64>(connection)?;
    let test_key_allowed =
        config.allow_test_plaintext_crypto_keys && test_plaintext_database_key_count > 0;
    let can_enable_encrypted_history = active_non_test_database_key_count > 0 || test_key_allowed;
    let action_required = if can_enable_encrypted_history {
        "none"
    } else if active_database_key_count == 0 {
        "register_active_database_key"
    } else {
        "replace_test_plaintext_key_with_os_protected_key"
    }
    .to_string();

    Ok(EncryptionCapabilityRecord {
        feature_gate_state,
        active_database_key_count,
        active_non_test_database_key_count,
        test_plaintext_database_key_count,
        unavailable_key_count,
        can_enable_encrypted_history,
        plaintext_fallback_allowed: false,
        key_material_exported: false,
        action_required,
    })
}

pub(super) fn ensure_no_open_critical_health_records(
    connection: &mut SqliteConnection,
    session_id: Option<&str>,
    operation_kind: &str,
) -> Result<(), TerminalPersistenceV2Error> {
    let mut query = terminal_data_health_records::table
        .filter(terminal_data_health_records::severity.eq("critical"))
        .filter(terminal_data_health_records::action_state.ne("resolved"))
        .filter(terminal_data_health_records::action_state.ne("ignored"))
        .into_boxed();
    if let Some(session_id) = session_id {
        query = query.filter(
            terminal_data_health_records::session_id
                .is_null()
                .or(terminal_data_health_records::session_id.eq(Some(session_id.to_string()))),
        );
    }

    let record = query
        .select(DataHealthRecordRow::as_select())
        .first::<DataHealthRecordRow>(connection)
        .optional()?;
    if let Some(record) = record {
        return Err(TerminalPersistenceV2Error::InvalidData(format!(
            "{operation_kind} is blocked by open critical data health record {}",
            record.id
        )));
    }

    Ok(())
}

pub(super) fn seed_payload_schemas(
    connection: &mut SqliteConnection,
    now: i64,
) -> Result<(), TerminalPersistenceV2Error> {
    let rows = vec![
        payload_schema_row(
            PAYLOAD_SCHEMA_UI_INPUT_V1,
            "journal_event_payload",
            "1.0.0",
            serde_json::json!({
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "title": "Terminal UI input journal payload",
                "type": "object",
                "required": ["data", "is_paste"],
                "properties": {
                    "data": { "type": "string" },
                    "is_paste": { "type": "boolean" }
                },
                "additionalProperties": false
            }),
            now,
        )?,
        payload_schema_row(
            PAYLOAD_SCHEMA_HISTORY_GAP_V1,
            "journal_event_payload",
            "1.0.0",
            serde_json::json!({
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "title": "Terminal history gap journal payload",
                "type": "object",
                "required": ["reason", "skipped_events", "estimated_dropped_bytes"],
                "properties": {
                    "reason": { "type": "string" },
                    "skipped_events": { "type": "integer", "minimum": 1 },
                    "estimated_dropped_bytes": { "type": ["integer", "null"], "minimum": 0 }
                },
                "additionalProperties": false
            }),
            now,
        )?,
        payload_schema_row(
            PAYLOAD_SCHEMA_JOURNAL_EVENT_V1,
            "journal_event_payload",
            "1.0.0",
            serde_json::json!({
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "title": "Generic terminal journal payload",
                "type": "object",
                "additionalProperties": true
            }),
            now,
        )?,
        payload_schema_row(
            PAYLOAD_SCHEMA_TOPOLOGY_SNAPSHOT_V1,
            "topology_snapshot_payload",
            "1.0.0",
            serde_json::json!({
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "title": "Terminal topology snapshot payload",
                "type": "object",
                "required": ["tabs"],
                "properties": {
                    "tabs": { "type": "array" }
                },
                "additionalProperties": true
            }),
            now,
        )?,
    ];

    for row in rows {
        insert_into(terminal_payload_schemas::table)
            .values(&row)
            .on_conflict(terminal_payload_schemas::id)
            .do_nothing()
            .execute(connection)?;
    }
    Ok(())
}

pub(super) fn payload_schema_row(
    id: &str,
    payload_kind: &str,
    schema_version: &str,
    schema: Value,
    created_at_ms: i64,
) -> Result<NewPayloadSchemaRow, TerminalPersistenceV2Error> {
    let schema_json = serde_json::to_string(&schema)?;
    let schema_hash = blake3_hash_text(&schema_json);
    Ok(NewPayloadSchemaRow {
        id: id.to_string(),
        payload_kind: payload_kind.to_string(),
        schema_version: schema_version.to_string(),
        schema_json,
        schema_hash,
        created_at_ms,
    })
}

pub(super) fn run_quick_check(
    connection: &mut SqliteConnection,
) -> Result<Vec<String>, TerminalPersistenceV2Error> {
    diesel::sql_query("PRAGMA quick_check")
        .load::<QuickCheckRow>(connection)
        .map(|rows| rows.into_iter().map(|row| row.quick_check).collect())
        .map_err(Into::into)
}

pub(super) fn run_passive_wal_checkpoint(
    connection: &mut SqliteConnection,
) -> Result<Value, TerminalPersistenceV2Error> {
    let rows =
        diesel::sql_query("PRAGMA wal_checkpoint(PASSIVE)").load::<WalCheckpointRow>(connection)?;
    let Some(row) = rows.into_iter().next() else {
        return Err(TerminalPersistenceV2Error::InvalidData(
            "wal_checkpoint returned no rows".to_string(),
        ));
    };

    Ok(serde_json::json!({
        "mode": "PASSIVE",
        "busy": row.busy,
        "log_frames": row.log,
        "checkpointed_frames": row.checkpointed,
    }))
}

pub(super) fn run_foreign_key_check(
    connection: &mut SqliteConnection,
) -> Result<Vec<ForeignKeyCheckRow>, TerminalPersistenceV2Error> {
    diesel::sql_query(
        "SELECT \"table\" AS table_name, rowid, parent, fkid FROM pragma_foreign_key_check",
    )
    .load::<ForeignKeyCheckRow>(connection)
    .map_err(Into::into)
}

pub(super) fn validate_history_checksums(
    connection: &mut SqliteConnection,
    session_id: Option<&str>,
) -> Result<HistoryValidation, TerminalPersistenceV2Error> {
    let mut failures = Vec::new();
    let schema_ids = terminal_payload_schemas::table
        .select(terminal_payload_schemas::id)
        .load::<String>(connection)?;

    let mut journal_query = terminal_journal_events::table.into_boxed();
    if let Some(session_id) = session_id {
        journal_query = journal_query.filter(terminal_journal_events::session_id.eq(session_id));
    }
    let journal_rows = journal_query
        .select((
            terminal_journal_events::id,
            terminal_journal_events::payload_json,
            terminal_journal_events::payload_schema_id,
        ))
        .load::<(String, Option<String>, Option<String>)>(connection)?;
    for (id, payload_json, payload_schema_id) in &journal_rows {
        validate_payload_schema_ref(
            "journal_event",
            id,
            payload_json.is_some(),
            payload_schema_id.as_deref(),
            &schema_ids,
            &mut failures,
        );
    }

    let mut segment_query = terminal_stream_segments::table.into_boxed();
    if let Some(session_id) = session_id {
        segment_query = segment_query.filter(terminal_stream_segments::session_id.eq(session_id));
    }
    let segment_rows =
        segment_query.select(StreamSegmentRow::as_select()).load::<StreamSegmentRow>(connection)?;
    for row in &segment_rows {
        validate_stream_segment_ranges(row, &mut failures);
        validate_checksum_bytes(
            "stream_segment",
            &row.id,
            &row.payload,
            &row.checksum_algorithm,
            &row.checksum,
            &mut failures,
        );
    }

    let mut screen_query = terminal_screen_snapshots::table.into_boxed();
    if let Some(session_id) = session_id {
        screen_query = screen_query.filter(terminal_screen_snapshots::session_id.eq(session_id));
    }
    let screen_rows = screen_query
        .select((
            terminal_screen_snapshots::id,
            terminal_screen_snapshots::screen_json,
            terminal_screen_snapshots::checksum_algorithm,
            terminal_screen_snapshots::checksum,
        ))
        .load::<(String, String, String, String)>(connection)?;
    for (id, payload, algorithm, checksum) in &screen_rows {
        validate_checksum_text("screen_snapshot", id, payload, algorithm, checksum, &mut failures);
    }

    let mut topology_query = terminal_topology_snapshots::table.into_boxed();
    if let Some(session_id) = session_id {
        topology_query =
            topology_query.filter(terminal_topology_snapshots::session_id.eq(session_id));
    }
    let topology_rows = topology_query
        .select((
            terminal_topology_snapshots::id,
            terminal_topology_snapshots::pane_high_water_json,
            terminal_topology_snapshots::topology_json,
            terminal_topology_snapshots::payload_schema_id,
            terminal_topology_snapshots::checksum_algorithm,
            terminal_topology_snapshots::checksum,
        ))
        .load::<(String, String, String, Option<String>, String, String)>(connection)?;
    for (id, pane_high_water_json, payload, payload_schema_id, algorithm, checksum) in
        &topology_rows
    {
        validate_payload_schema_ref(
            "topology_snapshot",
            id,
            true,
            payload_schema_id.as_deref(),
            &schema_ids,
            &mut failures,
        );
        validate_checksum_text(
            "topology_snapshot",
            id,
            payload,
            algorithm,
            checksum,
            &mut failures,
        );
        validate_topology_pane_high_water_json_payload(id, pane_high_water_json, &mut failures);
    }

    validate_sequence_invariants(connection, session_id, &mut failures)?;

    Ok(HistoryValidation {
        journal_events_checked: journal_rows.len(),
        stream_segments_checked: segment_rows.len(),
        screen_snapshots_checked: screen_rows.len(),
        topology_snapshots_checked: topology_rows.len(),
        failures,
    })
}

pub(super) fn persist_history_validation_health_records(
    connection: &mut SqliteConnection,
    session_id: Option<&str>,
    validation: &HistoryValidation,
    detected_at_ms: i64,
    evidence_ref: Option<&str>,
) -> Result<(), TerminalPersistenceV2Error> {
    for failure in &validation.failures {
        let detection_kind = if failure.contains("checksum mismatch") {
            "checksum_mismatch"
        } else if failure.contains("payload_schema_id") {
            "migration_mismatch"
        } else if failure.contains("topology high-water")
            || failure.contains("topology high_water_event_seq")
            || failure.contains("pane_high_water_json")
        {
            "projection_drift"
        } else if failure.starts_with("stream_cursor:")
            || failure.starts_with("pane:")
            || failure.starts_with("session_cursor:")
            || failure.starts_with("commit_log:")
            || failure.starts_with("stream_segment:")
        {
            "missing_segment"
        } else {
            "manual"
        };
        let is_canonical_replay_source =
            failure.starts_with("stream_segment:") || failure.starts_with("journal_event:");
        let severity = if is_canonical_replay_source { "critical" } else { "error" };
        let action_state =
            if is_canonical_replay_source { "quarantined" } else { "rebuild_pending" };
        let affected_ref = Some(failure.clone());
        let existing = terminal_data_health_records::table
            .filter(terminal_data_health_records::affected_ref.eq(affected_ref.clone()))
            .filter(terminal_data_health_records::detection_kind.eq(detection_kind))
            .filter(terminal_data_health_records::action_state.ne("resolved"))
            .filter(terminal_data_health_records::action_state.ne("ignored"))
            .select(DataHealthRecordRow::as_select())
            .first::<DataHealthRecordRow>(connection)
            .optional()?;
        if existing.is_some() {
            continue;
        }

        let details_json = Some(serde_json::to_string(&serde_json::json!({
            "failure": failure,
            "evidence_ref": evidence_ref,
            "validation": {
                "journal_events_checked": validation.journal_events_checked,
                "stream_segments_checked": validation.stream_segments_checked,
                "screen_snapshots_checked": validation.screen_snapshots_checked,
                "topology_snapshots_checked": validation.topology_snapshots_checked
            }
        }))?);
        let row = NewDataHealthRecordRow {
            id: new_id(),
            session_id: session_id.map(ToOwned::to_owned),
            pane_id: None,
            detection_kind: detection_kind.to_string(),
            severity: severity.to_string(),
            first_bad_event_seq: None,
            affected_ref,
            action_state: action_state.to_string(),
            detected_at_ms,
            resolved_at_ms: None,
            details_json,
            metadata_json: None,
        };
        insert_into(terminal_data_health_records::table).values(&row).execute(connection)?;
    }
    Ok(())
}

pub(super) fn load_latest_valid_screen_snapshot(
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

pub(super) fn load_latest_valid_topology_snapshot(
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

pub(super) fn screen_snapshot_hydration_failure(
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

pub(super) fn validate_screen_snapshot_topology_high_water(
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

pub(super) fn topology_snapshot_hydration_failure(
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

pub(super) fn validate_topology_pane_high_water_json(
    row: &TopologySnapshotRow,
    failures: &mut Vec<String>,
) {
    validate_topology_pane_high_water_json_payload(&row.id, &row.pane_high_water_json, failures);
}

pub(super) fn validate_topology_pane_high_water_json_payload(
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

pub(super) fn parse_pane_high_water_json(
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

pub(super) fn persist_projection_snapshot_failure(
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

pub(super) fn stream_segment_hydration_failure(row: &StreamSegmentRow) -> Option<String> {
    let mut failures = Vec::new();
    validate_stream_segment_ranges(row, &mut failures);
    validate_checksum_bytes(
        "stream_segment",
        &row.id,
        &row.payload,
        &row.checksum_algorithm,
        &row.checksum,
        &mut failures,
    );
    failures.into_iter().next()
}

pub(super) fn persist_hydration_segment_failure(
    connection: &mut SqliteConnection,
    session_id: &str,
    row: &StreamSegmentRow,
    failure: &str,
    detected_at_ms: i64,
) -> Result<(), TerminalPersistenceV2Error> {
    let validation = HistoryValidation {
        journal_events_checked: 0,
        stream_segments_checked: 1,
        screen_snapshots_checked: 0,
        topology_snapshots_checked: 0,
        failures: vec![failure.to_string()],
    };
    persist_history_validation_health_records(
        connection,
        Some(session_id),
        &validation,
        detected_at_ms,
        Some("hydrate_pane_history"),
    )?;

    let event_range_valid = row.event_seq_low <= row.event_seq_high;
    let byte_range_valid = row.byte_low < row.byte_high;
    let existing_gap = if event_range_valid {
        terminal_history_gaps::table
            .filter(terminal_history_gaps::session_id.eq(&row.session_id))
            .filter(terminal_history_gaps::pane_id.eq(Some(row.pane_id.clone())))
            .filter(terminal_history_gaps::stream_id.eq(&row.stream_id))
            .filter(terminal_history_gaps::gap_kind.eq("corrupted_segment"))
            .filter(terminal_history_gaps::event_seq_low.eq(Some(row.event_seq_low)))
            .filter(terminal_history_gaps::event_seq_high.eq(Some(row.event_seq_high)))
            .select(terminal_history_gaps::id)
            .first::<String>(connection)
            .optional()?
    } else {
        None
    };
    if existing_gap.is_some() {
        return Ok(());
    }

    let metadata_json = Some(serde_json::to_string(&serde_json::json!({
        "stream_segment_id": row.id,
        "failure": failure,
        "detected_by": "hydrate_pane_history"
    }))?);
    let gap = NewHistoryGapRow {
        id: new_id(),
        session_id: row.session_id.clone(),
        pane_id: Some(row.pane_id.clone()),
        stream_id: row.stream_id.clone(),
        gap_kind: "corrupted_segment".to_string(),
        event_seq_low: event_range_valid.then_some(row.event_seq_low),
        event_seq_high: event_range_valid.then_some(row.event_seq_high),
        byte_low: byte_range_valid.then_some(row.byte_low),
        byte_high: byte_range_valid.then_some(row.byte_high),
        estimated_dropped_bytes: byte_range_valid.then_some(row.byte_high - row.byte_low),
        estimated_dropped_events: event_range_valid
            .then_some(row.event_seq_high - row.event_seq_low + 1),
        reason: "canonical stream segment failed hydration validation".to_string(),
        writer_generation: Some(row.writer_generation.clone()),
        opened_at_ms: detected_at_ms,
        closed_at_ms: Some(detected_at_ms),
        metadata_json,
    };
    insert_into(terminal_history_gaps::table).values(&gap).execute(connection)?;
    Ok(())
}

pub(super) fn validate_checksum_bytes(
    row_kind: &str,
    id: &str,
    payload: &[u8],
    algorithm: &str,
    expected: &str,
    failures: &mut Vec<String>,
) {
    if algorithm != "blake3" {
        failures.push(format!("{row_kind}:{id} uses unsupported checksum algorithm {algorithm}"));
        return;
    }
    let actual = blake3_hash_bytes(payload);
    if actual != expected {
        failures.push(format!("{row_kind}:{id} checksum mismatch"));
    }
}

pub(super) fn validate_checksum_text(
    row_kind: &str,
    id: &str,
    payload: &str,
    algorithm: &str,
    expected: &str,
    failures: &mut Vec<String>,
) {
    validate_checksum_bytes(row_kind, id, payload.as_bytes(), algorithm, expected, failures);
}

pub(super) fn validate_payload_schema_ref(
    row_kind: &str,
    id: &str,
    payload_present: bool,
    payload_schema_id: Option<&str>,
    schema_ids: &[String],
    failures: &mut Vec<String>,
) {
    if !payload_present {
        return;
    }
    let Some(payload_schema_id) = payload_schema_id else {
        failures.push(format!("{row_kind}:{id} missing payload_schema_id"));
        return;
    };
    if !schema_ids.iter().any(|schema_id| schema_id == payload_schema_id) {
        failures.push(format!(
            "{row_kind}:{id} references unknown payload_schema_id {payload_schema_id}"
        ));
    }
}

pub(super) fn validate_stream_segment_ranges(row: &StreamSegmentRow, failures: &mut Vec<String>) {
    if row.event_seq_high < row.event_seq_low {
        failures.push(format!(
            "stream_segment:{} invalid event range {}..{}",
            row.id, row.event_seq_low, row.event_seq_high
        ));
    }
    if row.byte_high < row.byte_low {
        failures.push(format!(
            "stream_segment:{} invalid byte range {}..{}",
            row.id, row.byte_low, row.byte_high
        ));
        return;
    }
    let expected_payload_len = row.byte_high - row.byte_low;
    if row.payload_len != expected_payload_len {
        failures.push(format!(
            "stream_segment:{} payload_len={} expected={}",
            row.id, row.payload_len, expected_payload_len
        ));
    }
    if row.stored_byte_len != i64::try_from(row.payload.len()).unwrap_or(i64::MAX) {
        failures.push(format!(
            "stream_segment:{} stored_byte_len={} actual={}",
            row.id,
            row.stored_byte_len,
            row.payload.len()
        ));
    }
}

pub(super) fn validate_sequence_invariants(
    connection: &mut SqliteConnection,
    session_id: Option<&str>,
    failures: &mut Vec<String>,
) -> Result<(), TerminalPersistenceV2Error> {
    let mut cursor_query = terminal_stream_cursors::table.into_boxed();
    if let Some(session_id) = session_id {
        cursor_query = cursor_query.filter(terminal_stream_cursors::session_id.eq(session_id));
    }
    let cursors =
        cursor_query.select(StreamCursorRow::as_select()).load::<StreamCursorRow>(connection)?;
    for cursor in cursors {
        let segment_event_high = max_stream_segment_event_high(
            connection,
            &cursor.session_id,
            &cursor.pane_id,
            Some(&cursor.stream_id),
        )?;
        let journal_event_high = max_journal_event_seq(
            connection,
            &cursor.session_id,
            &cursor.pane_id,
            Some(&cursor.stream_id),
        )?;
        let gap_event_high = max_history_gap_event_high(
            connection,
            &cursor.session_id,
            &cursor.pane_id,
            Some(&cursor.stream_id),
        )?;
        if let Some(observed_event_high) =
            max_optional_i64(&[segment_event_high, journal_event_high, gap_event_high])
        {
            let expected_next_event_seq = observed_event_high + 1;
            if cursor.next_event_seq != expected_next_event_seq {
                failures.push(format!(
                    "stream_cursor:{} next_event_seq={} expected={}",
                    cursor.id, cursor.next_event_seq, expected_next_event_seq
                ));
            }
        }

        let expected_next_byte_seq = max_stream_segment_byte_high(
            connection,
            &cursor.session_id,
            &cursor.pane_id,
            Some(&cursor.stream_id),
        )?
        .unwrap_or(0);
        if cursor.next_byte_seq != expected_next_byte_seq {
            failures.push(format!(
                "stream_cursor:{} next_byte_seq={} expected={}",
                cursor.id, cursor.next_byte_seq, expected_next_byte_seq
            ));
        }
    }

    let mut pane_query = terminal_panes::table.into_boxed();
    if let Some(session_id) = session_id {
        pane_query = pane_query.filter(terminal_panes::session_id.eq(session_id));
    }
    let panes = pane_query
        .select((terminal_panes::id, terminal_panes::session_id, terminal_panes::last_event_seq))
        .load::<(String, String, i64)>(connection)?;
    for (pane_id, pane_session_id, last_event_seq) in panes {
        let segment_event_high =
            max_stream_segment_event_high(connection, &pane_session_id, &pane_id, None)?;
        let journal_event_high =
            max_journal_event_seq(connection, &pane_session_id, &pane_id, None)?;
        let gap_event_high =
            max_history_gap_event_high(connection, &pane_session_id, &pane_id, None)?;
        if let Some(expected_last_event_seq) =
            max_optional_i64(&[segment_event_high, journal_event_high, gap_event_high])
        {
            if last_event_seq != expected_last_event_seq {
                failures.push(format!(
                    "pane:{} last_event_seq={} expected={}",
                    pane_id, last_event_seq, expected_last_event_seq
                ));
            }
        }
    }

    validate_stream_segment_ordering(connection, session_id, failures)?;
    validate_commit_sequence_invariants(connection, session_id, failures)?;

    Ok(())
}

pub(super) fn validate_stream_segment_ordering(
    connection: &mut SqliteConnection,
    session_id: Option<&str>,
    failures: &mut Vec<String>,
) -> Result<(), TerminalPersistenceV2Error> {
    let mut query = terminal_stream_segments::table.into_boxed();
    if let Some(session_id) = session_id {
        query = query.filter(terminal_stream_segments::session_id.eq(session_id));
    }
    let rows = query
        .order((
            terminal_stream_segments::session_id.asc(),
            terminal_stream_segments::pane_id.asc(),
            terminal_stream_segments::stream_id.asc(),
            terminal_stream_segments::event_seq_low.asc(),
            terminal_stream_segments::byte_low.asc(),
        ))
        .select((
            terminal_stream_segments::id,
            terminal_stream_segments::session_id,
            terminal_stream_segments::pane_id,
            terminal_stream_segments::stream_id,
            terminal_stream_segments::event_seq_low,
            terminal_stream_segments::event_seq_high,
            terminal_stream_segments::byte_low,
            terminal_stream_segments::byte_high,
        ))
        .load::<(String, String, String, String, i64, i64, i64, i64)>(connection)?;

    let mut previous: Option<(String, String, String, String, i64, i64)> = None;
    for (
        id,
        row_session_id,
        pane_id,
        stream_id,
        event_seq_low,
        event_seq_high,
        byte_low,
        byte_high,
    ) in rows
    {
        if let Some((
            previous_id,
            previous_session_id,
            previous_pane_id,
            previous_stream_id,
            previous_event_high,
            previous_byte_high,
        )) = previous.as_ref()
        {
            if previous_session_id == &row_session_id
                && previous_pane_id == &pane_id
                && previous_stream_id == &stream_id
            {
                if event_seq_low <= *previous_event_high {
                    failures.push(format!(
                        "stream_segment:{id} overlaps stream_segment:{previous_id} event range"
                    ));
                }
                if byte_low < *previous_byte_high {
                    failures.push(format!(
                        "stream_segment:{id} overlaps stream_segment:{previous_id} byte range"
                    ));
                }
            }
        }
        previous = Some((id, row_session_id, pane_id, stream_id, event_seq_high, byte_high));
    }

    Ok(())
}

pub(super) fn validate_commit_sequence_invariants(
    connection: &mut SqliteConnection,
    session_id: Option<&str>,
    failures: &mut Vec<String>,
) -> Result<(), TerminalPersistenceV2Error> {
    let mut cursor_query = terminal_session_cursors::table.into_boxed();
    if let Some(session_id) = session_id {
        cursor_query = cursor_query.filter(terminal_session_cursors::session_id.eq(session_id));
    }
    let cursors = cursor_query
        .select((terminal_session_cursors::session_id, terminal_session_cursors::next_commit_seq))
        .load::<(String, i64)>(connection)?;
    for (cursor_session_id, next_commit_seq) in cursors {
        let max_commit_seq = terminal_commit_log::table
            .filter(terminal_commit_log::session_id.eq(&cursor_session_id))
            .select(max(terminal_commit_log::commit_seq))
            .first::<Option<i64>>(connection)?
            .unwrap_or(0);
        let expected_next_commit_seq = max_commit_seq + 1;
        if next_commit_seq != expected_next_commit_seq {
            failures.push(format!(
                "session_cursor:{cursor_session_id} next_commit_seq={next_commit_seq} expected={expected_next_commit_seq}"
            ));
        }
    }

    let mut commit_query = terminal_commit_log::table.into_boxed();
    if let Some(session_id) = session_id {
        commit_query = commit_query.filter(terminal_commit_log::session_id.eq(session_id));
    }
    let commits = commit_query
        .order((terminal_commit_log::session_id.asc(), terminal_commit_log::commit_seq.asc()))
        .select((
            terminal_commit_log::id,
            terminal_commit_log::session_id,
            terminal_commit_log::commit_seq,
        ))
        .load::<(String, String, i64)>(connection)?;

    let mut previous_session: Option<String> = None;
    let mut expected_commit_seq = 1_i64;
    for (commit_id, commit_session_id, commit_seq) in commits {
        if previous_session.as_deref() != Some(commit_session_id.as_str()) {
            previous_session = Some(commit_session_id.clone());
            expected_commit_seq = 1;
        }
        if commit_seq != expected_commit_seq {
            failures.push(format!(
                "commit_log:{commit_id} commit_seq={commit_seq} expected={expected_commit_seq}"
            ));
            expected_commit_seq = commit_seq + 1;
        } else {
            expected_commit_seq += 1;
        }
    }

    Ok(())
}

pub(super) fn max_stream_segment_event_high(
    connection: &mut SqliteConnection,
    session_id: &str,
    pane_id: &str,
    stream_id: Option<&str>,
) -> Result<Option<i64>, TerminalPersistenceV2Error> {
    let mut query = terminal_stream_segments::table
        .filter(terminal_stream_segments::session_id.eq(session_id))
        .filter(terminal_stream_segments::pane_id.eq(pane_id))
        .into_boxed();
    if let Some(stream_id) = stream_id {
        query = query.filter(terminal_stream_segments::stream_id.eq(stream_id));
    }
    query
        .select(max(terminal_stream_segments::event_seq_high))
        .first::<Option<i64>>(connection)
        .map_err(Into::into)
}

pub(super) fn max_stream_segment_byte_high(
    connection: &mut SqliteConnection,
    session_id: &str,
    pane_id: &str,
    stream_id: Option<&str>,
) -> Result<Option<i64>, TerminalPersistenceV2Error> {
    let mut query = terminal_stream_segments::table
        .filter(terminal_stream_segments::session_id.eq(session_id))
        .filter(terminal_stream_segments::pane_id.eq(pane_id))
        .into_boxed();
    if let Some(stream_id) = stream_id {
        query = query.filter(terminal_stream_segments::stream_id.eq(stream_id));
    }
    query
        .select(max(terminal_stream_segments::byte_high))
        .first::<Option<i64>>(connection)
        .map_err(Into::into)
}

pub(super) fn max_journal_event_seq(
    connection: &mut SqliteConnection,
    session_id: &str,
    pane_id: &str,
    stream_id: Option<&str>,
) -> Result<Option<i64>, TerminalPersistenceV2Error> {
    let mut query = terminal_journal_events::table
        .filter(terminal_journal_events::session_id.eq(session_id))
        .filter(terminal_journal_events::pane_id.eq(Some(pane_id.to_string())))
        .into_boxed();
    if let Some(stream_id) = stream_id {
        query = query.filter(terminal_journal_events::stream_id.eq(stream_id));
    }
    query
        .select(max(terminal_journal_events::event_seq))
        .first::<Option<i64>>(connection)
        .map_err(Into::into)
}

pub(super) fn max_history_gap_event_high(
    connection: &mut SqliteConnection,
    session_id: &str,
    pane_id: &str,
    stream_id: Option<&str>,
) -> Result<Option<i64>, TerminalPersistenceV2Error> {
    let mut query = terminal_history_gaps::table
        .filter(terminal_history_gaps::session_id.eq(session_id))
        .filter(terminal_history_gaps::pane_id.eq(Some(pane_id.to_string())))
        .into_boxed();
    if let Some(stream_id) = stream_id {
        query = query.filter(terminal_history_gaps::stream_id.eq(stream_id));
    }
    query
        .select(max(terminal_history_gaps::event_seq_high))
        .first::<Option<i64>>(connection)
        .map_err(Into::into)
}

pub(super) fn max_optional_i64(values: &[Option<i64>]) -> Option<i64> {
    values.iter().filter_map(|value| *value).max()
}
