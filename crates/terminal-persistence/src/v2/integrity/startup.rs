use super::super::*;

pub(in crate::v2) fn verify_seeded_defaults(
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

pub(in crate::v2) fn load_feature_gate_state(
    connection: &mut SqliteConnection,
    name: FeatureGateName,
) -> Result<FeatureGateState, TerminalPersistenceV2Error> {
    let row = terminal_feature_gates::table
        .filter(terminal_feature_gates::feature_name.eq(name.as_str()))
        .select(FeatureGateRow::as_select())
        .first::<FeatureGateRow>(connection)?;
    FeatureGateState::parse(&row.state)
}

pub(in crate::v2) fn validate_feature_gate_transition(
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

pub(in crate::v2) fn enforce_encryption_startup_policy(
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

pub(in crate::v2) fn encryption_capability_state_for_connection(
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

pub(in crate::v2) fn ensure_no_open_critical_health_records(
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

pub(in crate::v2) fn seed_payload_schemas(
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

pub(in crate::v2) fn payload_schema_row(
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
