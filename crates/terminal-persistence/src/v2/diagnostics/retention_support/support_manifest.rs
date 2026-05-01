use super::{super::super::*, support_facts::SupportBundleDiagnosticFacts};

pub(super) fn build_support_bundle_manifest(
    db_path: &Path,
    bundle: &SupportBundleRow,
    now: i64,
    scope_hash: &str,
    include_raw: bool,
    facts: SupportBundleDiagnosticFacts,
) -> Value {
    serde_json::json!({
        "support_bundle_id": bundle.id,
        "generated_at_ms": now,
        "scope_hash": scope_hash,
        "scope_value_stored_in_bundle_row_only": true,
        "redaction_profile_id": bundle.redaction_profile_id,
        "include_raw": include_raw,
        "raw_content_included": include_raw,
        "raw_content_included_by_default": false,
        "excluded_classes": excluded_classes(include_raw),
        "included_classes": included_classes(include_raw),
        "raw_terminal_output_rows_serialized": false,
        "raw_command_text_rows_serialized": false,
        "raw_paths_serialized": false,
        "crypto_key_refs_serialized": false,
        "db_path_hash": path_hash(db_path),
        "wal_path_hash": path_hash(&sqlite_sidecar_path(db_path, "-wal")),
        "storage": {
            "db_file_bytes": facts.db_file_bytes,
            "wal_file_bytes": facts.wal_file_bytes,
        },
        "counts": {
            "sessions": facts.session_count,
            "panes": facts.pane_count,
            "stream_segments": facts.stream_segment_count,
            "command_history_entries": facts.command_history_count,
            "search_documents": facts.search_document_count,
            "external_artifacts": facts.external_artifact_count,
        },
        "data_health": {
            "open_record_count": facts.open_health_count,
            "open_critical_record_count": facts.open_critical_health_count,
        },
        "restore_drills": {
            "passed_count": facts.restore_drill_passed_count,
            "failed_count": facts.restore_drill_failed_count,
            "latest_status": facts.latest_restore_drill_status,
        },
        "feature_gates": facts.feature_gates,
        "outbox": facts.outbox,
        "compression": facts.compression,
        "retention": facts.retention,
        "encryption": facts.encryption,
        "prompt_injection_text_is_data": true,
        "historical_replay_side_effects_suppressed": true,
    })
}

fn excluded_classes(include_raw: bool) -> Vec<&'static str> {
    if include_raw {
        vec!["class_secret_material"]
    } else {
        vec!["class_sensitive_content", "class_secret_material"]
    }
}

fn included_classes(include_raw: bool) -> Vec<&'static str> {
    if include_raw {
        vec![
            "class_public_diagnostic",
            "class_local_metadata",
            "class_user_context",
            "class_sensitive_content",
        ]
    } else {
        vec!["class_public_diagnostic", "class_local_metadata", "class_user_context_redacted"]
    }
}
