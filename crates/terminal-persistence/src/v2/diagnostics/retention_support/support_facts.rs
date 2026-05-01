use super::{super::super::*, retention::collect_retention_diagnostics};

pub(super) struct SupportBundleDiagnosticFacts {
    pub(super) feature_gates: Vec<Value>,
    pub(super) open_health_count: i64,
    pub(super) open_critical_health_count: i64,
    pub(super) restore_drill_passed_count: i64,
    pub(super) restore_drill_failed_count: i64,
    pub(super) latest_restore_drill_status: Option<String>,
    pub(super) outbox: OutboxDiagnosticsRecord,
    pub(super) compression: CompressionDiagnosticsRecord,
    pub(super) retention: RetentionDiagnosticsRecord,
    pub(super) encryption: EncryptionCapabilityRecord,
    pub(super) session_count: i64,
    pub(super) pane_count: i64,
    pub(super) stream_segment_count: i64,
    pub(super) command_history_count: i64,
    pub(super) search_document_count: i64,
    pub(super) external_artifact_count: i64,
    pub(super) db_file_bytes: Option<i64>,
    pub(super) wal_file_bytes: Option<i64>,
}

pub(super) fn collect_support_bundle_facts(
    connection: &mut SqliteConnection,
    db_path: &Path,
    config: &TerminalPersistenceV2Config,
    now: i64,
) -> Result<SupportBundleDiagnosticFacts, TerminalPersistenceV2Error> {
    Ok(SupportBundleDiagnosticFacts {
        feature_gates: load_feature_gate_diagnostics(connection)?,
        open_health_count: count_open_health_records(connection, None)?,
        open_critical_health_count: count_open_health_records(connection, Some("critical"))?,
        restore_drill_passed_count: count_restore_drills(connection, "passed")?,
        restore_drill_failed_count: count_restore_drills(connection, "failed")?,
        latest_restore_drill_status: latest_restore_drill_status(connection)?,
        outbox: collect_outbox_diagnostics(connection, now)?,
        compression: collect_compression_diagnostics(connection, now)?,
        retention: collect_retention_diagnostics(connection, now, None)?,
        encryption: encryption_capability_state_for_connection(connection, config)?,
        session_count: terminal_sessions::table.count().get_result::<i64>(connection)?,
        pane_count: terminal_panes::table.count().get_result::<i64>(connection)?,
        stream_segment_count: terminal_stream_segments::table
            .count()
            .get_result::<i64>(connection)?,
        command_history_count: terminal_command_history_entries::table
            .count()
            .get_result::<i64>(connection)?,
        search_document_count: terminal_search_documents::table
            .count()
            .get_result::<i64>(connection)?,
        external_artifact_count: terminal_external_artifacts::table
            .count()
            .get_result::<i64>(connection)?,
        db_file_bytes: file_len_i64(db_path)?,
        wal_file_bytes: file_len_i64(&sqlite_sidecar_path(db_path, "-wal"))?,
    })
}

fn load_feature_gate_diagnostics(
    connection: &mut SqliteConnection,
) -> Result<Vec<Value>, TerminalPersistenceV2Error> {
    terminal_feature_gates::table
        .select((
            terminal_feature_gates::feature_name,
            terminal_feature_gates::state,
            terminal_feature_gates::rollout_scope,
        ))
        .order(terminal_feature_gates::feature_name.asc())
        .load::<(String, String, String)>(connection)
        .map(|rows| {
            rows.into_iter()
                .map(|(feature_name, state, scope)| {
                    serde_json::json!({
                        "feature_name": feature_name,
                        "state": state,
                        "scope": scope,
                    })
                })
                .collect()
        })
        .map_err(Into::into)
}

fn count_open_health_records(
    connection: &mut SqliteConnection,
    severity: Option<&str>,
) -> Result<i64, TerminalPersistenceV2Error> {
    let mut query = terminal_data_health_records::table.into_boxed();
    if let Some(severity) = severity {
        query = query.filter(terminal_data_health_records::severity.eq(severity));
    }
    query
        .filter(terminal_data_health_records::action_state.ne("resolved"))
        .filter(terminal_data_health_records::action_state.ne("ignored"))
        .count()
        .get_result::<i64>(connection)
        .map_err(Into::into)
}

fn count_restore_drills(
    connection: &mut SqliteConnection,
    result: &str,
) -> Result<i64, TerminalPersistenceV2Error> {
    terminal_restore_drills::table
        .filter(terminal_restore_drills::result.eq(result))
        .count()
        .get_result::<i64>(connection)
        .map_err(Into::into)
}

fn latest_restore_drill_status(
    connection: &mut SqliteConnection,
) -> Result<Option<String>, TerminalPersistenceV2Error> {
    terminal_restore_drills::table
        .order(terminal_restore_drills::checked_at_ms.desc())
        .select(terminal_restore_drills::result)
        .first::<String>(connection)
        .optional()
        .map_err(Into::into)
}
