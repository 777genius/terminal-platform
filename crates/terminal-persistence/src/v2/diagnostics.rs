use super::*;

pub(super) fn collect_outbox_diagnostics(
    connection: &mut SqliteConnection,
    now: i64,
) -> Result<OutboxDiagnosticsRecord, TerminalPersistenceV2Error> {
    let pending_count = count_outbox_state(connection, "pending")?;
    let due_pending_count = terminal_outbox_messages::table
        .filter(terminal_outbox_messages::state.eq("pending"))
        .filter(terminal_outbox_messages::next_run_at_ms.le(now))
        .count()
        .get_result::<i64>(connection)?;
    let claimed_count = count_outbox_state(connection, "claimed")?;
    let stale_claim_count = terminal_outbox_messages::table
        .filter(terminal_outbox_messages::state.eq("claimed"))
        .filter(terminal_outbox_messages::claimed_until_ms.le(Some(now)))
        .count()
        .get_result::<i64>(connection)?;
    let done_count = count_outbox_state(connection, "done")?;
    let failed_count = count_outbox_state(connection, "failed")?;
    let quarantined_count = count_outbox_state(connection, "quarantined")?;
    let oldest_due_pending_created_at = terminal_outbox_messages::table
        .filter(terminal_outbox_messages::state.eq("pending"))
        .filter(terminal_outbox_messages::next_run_at_ms.le(now))
        .select(min(terminal_outbox_messages::created_at_ms))
        .first::<Option<i64>>(connection)?;
    let next_pending_run_at = terminal_outbox_messages::table
        .filter(terminal_outbox_messages::state.eq("pending"))
        .filter(terminal_outbox_messages::next_run_at_ms.gt(now))
        .select(min(terminal_outbox_messages::next_run_at_ms))
        .first::<Option<i64>>(connection)?;

    Ok(OutboxDiagnosticsRecord {
        generated_at_ms: now,
        pending_count,
        due_pending_count,
        claimed_count,
        stale_claim_count,
        done_count,
        failed_count,
        quarantined_count,
        oldest_due_pending_age_ms: oldest_due_pending_created_at
            .map(|created_at_ms| (now - created_at_ms).max(0)),
        next_pending_due_in_ms: next_pending_run_at.map(|run_at_ms| (run_at_ms - now).max(0)),
    })
}

pub(super) fn count_outbox_state(
    connection: &mut SqliteConnection,
    state_name: &str,
) -> Result<i64, TerminalPersistenceV2Error> {
    Ok(terminal_outbox_messages::table
        .filter(terminal_outbox_messages::state.eq(state_name))
        .count()
        .get_result::<i64>(connection)?)
}

pub(super) fn collect_compression_diagnostics(
    connection: &mut SqliteConnection,
    now: i64,
) -> Result<CompressionDiagnosticsRecord, TerminalPersistenceV2Error> {
    let feature_gate_state = terminal_feature_gates::table
        .filter(
            terminal_feature_gates::feature_name
                .eq(FeatureGateName::SegmentCompressionZstd.as_str()),
        )
        .select(terminal_feature_gates::state)
        .first::<String>(connection)
        .optional()?
        .unwrap_or_else(|| FeatureGateState::Disabled.as_str().to_string());
    let raw_segment_count = count_stream_segments_by_compression(connection, "none")?;
    let zstd_segment_count = count_stream_segments_by_compression(connection, "zstd")?;
    let unsupported_segment_count = terminal_stream_segments::table
        .filter(terminal_stream_segments::compression.ne("none"))
        .filter(terminal_stream_segments::compression.ne("zstd"))
        .count()
        .get_result::<i64>(connection)?;
    let rewrite_candidate_count = if feature_gate_state == FeatureGateState::Enabled.as_str() {
        raw_segment_count
    } else {
        0
    };
    let action_taken = if feature_gate_state == FeatureGateState::Enabled.as_str() {
        "skipped_restore_drill_guard"
    } else {
        "skipped_feature_disabled"
    }
    .to_string();

    Ok(CompressionDiagnosticsRecord {
        generated_at_ms: now,
        feature_gate_state,
        raw_segment_count,
        zstd_segment_count,
        unsupported_segment_count,
        rewrite_candidate_count,
        segments_rewritten: 0,
        restore_drill_required: true,
        action_taken,
    })
}

pub(super) fn count_stream_segments_by_compression(
    connection: &mut SqliteConnection,
    compression: &str,
) -> Result<i64, TerminalPersistenceV2Error> {
    Ok(terminal_stream_segments::table
        .filter(terminal_stream_segments::compression.eq(compression))
        .count()
        .get_result::<i64>(connection)?)
}

#[derive(Debug, Clone)]
pub(super) struct InsertedAiContextItem {
    pub(super) id: String,
    pub(super) content_preview: String,
}

pub(super) fn insert_ai_context_items_from_command_history(
    connection: &mut SqliteConnection,
    package_id: &str,
    session_id: Option<&str>,
    pane_id: Option<&str>,
    limit: i64,
) -> Result<Vec<InsertedAiContextItem>, TerminalPersistenceV2Error> {
    if limit <= 0 {
        return Ok(Vec::new());
    }
    let mut query = terminal_command_history_entries::table.into_boxed();
    if let Some(session_id) = session_id {
        query = query
            .filter(terminal_command_history_entries::session_id.eq(Some(session_id.to_string())));
    }
    if let Some(pane_id) = pane_id {
        query =
            query.filter(terminal_command_history_entries::pane_id.eq(Some(pane_id.to_string())));
    }
    let rows = query
        .order(terminal_command_history_entries::last_used_at_ms.desc())
        .limit(limit)
        .select((
            terminal_command_history_entries::id,
            terminal_command_history_entries::session_id,
            terminal_command_history_entries::pane_id,
            terminal_command_history_entries::command_block_id,
            terminal_command_history_entries::display_text,
            terminal_command_history_entries::redacted_text,
            terminal_command_history_entries::redaction_state,
            terminal_command_history_entries::trust_level,
            terminal_command_history_entries::rerun_policy,
        ))
        .load::<(
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            String,
            Option<String>,
            String,
            String,
            String,
        )>(connection)?;

    let mut inserted = Vec::new();
    for (
        source_id,
        session_id,
        pane_id,
        command_block_id,
        display_text,
        redacted_text,
        redaction_state,
        trust_level,
        rerun_policy,
    ) in rows
    {
        let preview_source = redacted_text.as_deref().unwrap_or(&display_text);
        let content_preview = limit_text_preview(&redact_terminal_text(preview_source), 512);
        let row = NewAiContextItemRow {
            id: new_id(),
            package_id: package_id.to_string(),
            source_kind: "command_history".to_string(),
            source_ref: Some(source_id),
            session_id,
            pane_id,
            command_block_id,
            event_seq_low: None,
            event_seq_high: None,
            byte_low: None,
            byte_high: None,
            redaction_state,
            data_only: 1,
            content_preview,
            metadata_json: Some(serde_json::to_string(&serde_json::json!({
                "source": "command_history",
                "trust_level": trust_level,
                "rerun_policy": rerun_policy,
                "raw_command_text_included": false,
                "command_hash_exported": false
            }))?),
        };
        insert_into(terminal_ai_context_items::table).values(&row).execute(connection)?;
        inserted.push(InsertedAiContextItem { id: row.id, content_preview: row.content_preview });
    }
    Ok(inserted)
}

pub(super) fn insert_ai_context_items_from_search_documents(
    connection: &mut SqliteConnection,
    package_id: &str,
    session_id: Option<&str>,
    pane_id: Option<&str>,
    limit: i64,
) -> Result<Vec<InsertedAiContextItem>, TerminalPersistenceV2Error> {
    if limit <= 0 {
        return Ok(Vec::new());
    }
    let mut query = terminal_search_documents::table.into_boxed();
    if let Some(session_id) = session_id {
        query = query.filter(terminal_search_documents::session_id.eq(session_id.to_string()));
    }
    if let Some(pane_id) = pane_id {
        query = query.filter(terminal_search_documents::pane_id.eq(Some(pane_id.to_string())));
    }
    let rows = query
        .order(terminal_search_documents::updated_at_ms.desc())
        .limit(limit)
        .select(SearchDocumentRow::as_select())
        .load::<SearchDocumentRow>(connection)?;

    let mut inserted = Vec::new();
    for document in rows {
        let content_preview = limit_text_preview(&document.text_preview, 512);
        let row = NewAiContextItemRow {
            id: new_id(),
            package_id: package_id.to_string(),
            source_kind: "search_document".to_string(),
            source_ref: Some(document.document_id),
            session_id: Some(document.session_id),
            pane_id: document.pane_id,
            command_block_id: document.command_block_id,
            event_seq_low: document.event_seq_low,
            event_seq_high: document.event_seq_high,
            byte_low: document.byte_low,
            byte_high: document.byte_high,
            redaction_state: document.redaction_state,
            data_only: 1,
            content_preview,
            metadata_json: Some(serde_json::to_string(&serde_json::json!({
                "source": "search_document",
                "document_kind": document.document_kind,
                "raw_terminal_output_included": false,
                "source_hash_exported": false
            }))?),
        };
        insert_into(terminal_ai_context_items::table).values(&row).execute(connection)?;
        inserted.push(InsertedAiContextItem { id: row.id, content_preview: row.content_preview });
    }
    Ok(inserted)
}

pub(super) fn insert_prompt_injection_findings_for_items(
    connection: &mut SqliteConnection,
    package_id: &str,
    items: &[InsertedAiContextItem],
    now: i64,
) -> Result<i64, TerminalPersistenceV2Error> {
    let mut count = 0_i64;
    for item in items {
        if let Some(pattern_kind) = detect_prompt_injection_pattern(&item.content_preview) {
            let finding = NewPromptInjectionFindingRow {
                id: new_id(),
                package_id: Some(package_id.to_string()),
                item_id: Some(item.id.clone()),
                severity: "warning".to_string(),
                pattern_kind: pattern_kind.to_string(),
                action_state: "detected".to_string(),
                detected_at_ms: now,
                evidence_preview: limit_text_preview(&item.content_preview, 160),
                metadata_json: Some(serde_json::to_string(&serde_json::json!({
                    "terminal_output_is_data_only": true,
                    "auto_action_allowed": false
                }))?),
            };
            insert_into(terminal_prompt_injection_findings::table)
                .values(&finding)
                .execute(connection)?;
            count += 1;
        }
    }
    Ok(count)
}

pub(super) fn collect_restore_replay_safety(
    connection: &mut SqliteConnection,
    session_id: &str,
) -> Result<RestoreReplaySafetyRecord, TerminalPersistenceV2Error> {
    let segments = terminal_stream_segments::table
        .filter(terminal_stream_segments::session_id.eq(session_id))
        .select(terminal_stream_segments::payload)
        .load::<Vec<u8>>(connection)?;
    let mut record = RestoreReplaySafetyRecord {
        session_id: session_id.to_string(),
        scanned_segment_count: i64::try_from(segments.len()).unwrap_or(i64::MAX),
        osc52_clipboard_count: 0,
        title_sequence_count: 0,
        hyperlink_sequence_count: 0,
        cwd_sequence_count: 0,
        shell_marker_sequence_count: 0,
        bel_byte_count: 0,
        side_effects_suppressed: true,
        prompt_injection_text_is_data: true,
    };
    for payload in segments {
        record.osc52_clipboard_count += count_byte_pattern(&payload, b"\x1b]52;");
        record.title_sequence_count +=
            count_byte_pattern(&payload, b"\x1b]0;") + count_byte_pattern(&payload, b"\x1b]2;");
        record.hyperlink_sequence_count += count_byte_pattern(&payload, b"\x1b]8;");
        record.cwd_sequence_count += count_byte_pattern(&payload, b"\x1b]7;");
        record.shell_marker_sequence_count +=
            count_byte_pattern(&payload, b"\x1b]133;") + count_byte_pattern(&payload, b"\x1b]633;");
        record.bel_byte_count += payload.iter().filter(|byte| **byte == 0x07).count() as i64;
    }
    Ok(record)
}

pub(super) fn count_byte_pattern(haystack: &[u8], needle: &[u8]) -> i64 {
    if needle.is_empty() || haystack.len() < needle.len() {
        return 0;
    }
    haystack.windows(needle.len()).filter(|window| *window == needle).count() as i64
}

pub(super) fn collect_retention_diagnostics(
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

pub(super) fn build_support_bundle_diagnostics(
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
