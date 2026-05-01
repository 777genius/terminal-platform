use super::super::*;
use super::*;

pub(in crate::v2) fn collect_retention_diagnostics(
    connection: &mut SqliteConnection,
    now: i64,
    selected_policy_id: Option<&str>,
) -> Result<RetentionDiagnosticsRecord, TerminalPersistenceV2Error> {
    let policy_id = selected_policy_id.unwrap_or(DEFAULT_RETENTION_POLICY_ID);
    let (policy_id, policy_kind, pressure_behavior, raw_history_prune_behavior) =
        terminal_retention_policies::table
            .filter(terminal_retention_policies::id.eq(policy_id))
            .select((
                terminal_retention_policies::id,
                terminal_retention_policies::policy_kind,
                terminal_retention_policies::pressure_behavior,
                terminal_retention_policies::raw_history_prune_behavior,
            ))
            .first::<(String, String, String, String)>(connection)?;
    let sessions_scanned = terminal_sessions::table
        .filter(terminal_sessions::retention_policy_id.eq(&policy_id))
        .count()
        .get_result::<i64>(connection)?;
    let action_taken = match (pressure_behavior.as_str(), raw_history_prune_behavior.as_str()) {
        ("warn_only", "never_silent") => "warn_only_no_delete",
        (_, "request_only") => "warn_only_delete_request_required",
        _ => "warn_only_no_silent_delete",
    }
    .to_string();

    Ok(RetentionDiagnosticsRecord {
        generated_at_ms: now,
        policy_id,
        policy_kind,
        pressure_behavior,
        raw_history_prune_behavior,
        sessions_scanned,
        scan_mode: "warn_only".to_string(),
        maintenance_deletes_raw_history: false,
        action_taken,
    })
}

pub(in crate::v2) fn build_support_bundle_diagnostics(
    connection: &mut SqliteConnection,
    db_path: &Path,
    config: &TerminalPersistenceV2Config,
    bundle: &SupportBundleRow,
    now: i64,
) -> Result<SupportBundleDiagnosticsRecord, TerminalPersistenceV2Error> {
    let scope_hash = blake3_hash_text(&bundle.scope_json);
    let feature_gates = terminal_feature_gates::table
        .select((
            terminal_feature_gates::feature_name,
            terminal_feature_gates::state,
            terminal_feature_gates::rollout_scope,
        ))
        .order(terminal_feature_gates::feature_name.asc())
        .load::<(String, String, String)>(connection)?
        .into_iter()
        .map(|(feature_name, state, scope)| {
            serde_json::json!({
                "feature_name": feature_name,
                "state": state,
                "scope": scope,
            })
        })
        .collect::<Vec<_>>();
    let open_health_count = terminal_data_health_records::table
        .filter(terminal_data_health_records::action_state.ne("resolved"))
        .filter(terminal_data_health_records::action_state.ne("ignored"))
        .count()
        .get_result::<i64>(connection)?;
    let open_critical_health_count = terminal_data_health_records::table
        .filter(terminal_data_health_records::severity.eq("critical"))
        .filter(terminal_data_health_records::action_state.ne("resolved"))
        .filter(terminal_data_health_records::action_state.ne("ignored"))
        .count()
        .get_result::<i64>(connection)?;
    let restore_drill_passed_count = terminal_restore_drills::table
        .filter(terminal_restore_drills::result.eq("passed"))
        .count()
        .get_result::<i64>(connection)?;
    let restore_drill_failed_count = terminal_restore_drills::table
        .filter(terminal_restore_drills::result.eq("failed"))
        .count()
        .get_result::<i64>(connection)?;
    let latest_restore_drill_status = terminal_restore_drills::table
        .order(terminal_restore_drills::checked_at_ms.desc())
        .select(terminal_restore_drills::result)
        .first::<String>(connection)
        .optional()?;
    let outbox = collect_outbox_diagnostics(connection, now)?;
    let compression = collect_compression_diagnostics(connection, now)?;
    let retention = collect_retention_diagnostics(connection, now, None)?;
    let encryption = encryption_capability_state_for_connection(connection, config)?;
    let session_count = terminal_sessions::table.count().get_result::<i64>(connection)?;
    let pane_count = terminal_panes::table.count().get_result::<i64>(connection)?;
    let stream_segment_count =
        terminal_stream_segments::table.count().get_result::<i64>(connection)?;
    let command_history_count =
        terminal_command_history_entries::table.count().get_result::<i64>(connection)?;
    let search_document_count =
        terminal_search_documents::table.count().get_result::<i64>(connection)?;
    let external_artifact_count =
        terminal_external_artifacts::table.count().get_result::<i64>(connection)?;
    let db_file_bytes = file_len_i64(db_path)?;
    let wal_file_bytes = file_len_i64(&sqlite_sidecar_path(db_path, "-wal"))?;

    let include_raw = bundle.include_raw != 0;
    let manifest = serde_json::json!({
        "support_bundle_id": bundle.id,
        "generated_at_ms": now,
        "scope_hash": scope_hash,
        "scope_value_stored_in_bundle_row_only": true,
        "redaction_profile_id": bundle.redaction_profile_id,
        "include_raw": include_raw,
        "raw_content_included": include_raw,
        "raw_content_included_by_default": false,
        "excluded_classes": if include_raw {
            vec!["class_secret_material"]
        } else {
            vec!["class_sensitive_content", "class_secret_material"]
        },
        "included_classes": if include_raw {
            vec!["class_public_diagnostic", "class_local_metadata", "class_user_context", "class_sensitive_content"]
        } else {
            vec!["class_public_diagnostic", "class_local_metadata", "class_user_context_redacted"]
        },
        "raw_terminal_output_rows_serialized": false,
        "raw_command_text_rows_serialized": false,
        "raw_paths_serialized": false,
        "crypto_key_refs_serialized": false,
        "db_path_hash": path_hash(db_path),
        "wal_path_hash": path_hash(&sqlite_sidecar_path(db_path, "-wal")),
        "storage": {
            "db_file_bytes": db_file_bytes,
            "wal_file_bytes": wal_file_bytes,
        },
        "counts": {
            "sessions": session_count,
            "panes": pane_count,
            "stream_segments": stream_segment_count,
            "command_history_entries": command_history_count,
            "search_documents": search_document_count,
            "external_artifacts": external_artifact_count,
        },
        "data_health": {
            "open_record_count": open_health_count,
            "open_critical_record_count": open_critical_health_count,
        },
        "restore_drills": {
            "passed_count": restore_drill_passed_count,
            "failed_count": restore_drill_failed_count,
            "latest_status": latest_restore_drill_status,
        },
        "feature_gates": feature_gates,
        "outbox": outbox,
        "compression": compression,
        "retention": retention,
        "encryption": encryption,
        "prompt_injection_text_is_data": true,
        "historical_replay_side_effects_suppressed": true,
    });

    Ok(SupportBundleDiagnosticsRecord {
        support_bundle_id: bundle.id.clone(),
        generated_at_ms: now,
        include_raw,
        raw_content_included: include_raw,
        manifest_json: manifest,
    })
}
