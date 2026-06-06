use super::super::super::*;

pub(in crate::v2) fn collect_retention_diagnostics(
    connection: &mut SqliteConnection,
    now: i64,
    selected_policy_id: Option<&str>,
) -> Result<RetentionDiagnosticsRecord, TerminalPersistenceV2Error> {
    let policy_id = selected_policy_id.unwrap_or(DEFAULT_RETENTION_POLICY_ID);
    let policy = load_retention_policy(connection, policy_id)?;
    let sessions_scanned = terminal_sessions::table
        .filter(terminal_sessions::retention_policy_id.eq(&policy.id))
        .count()
        .get_result::<i64>(connection)?;

    Ok(RetentionDiagnosticsRecord {
        generated_at_ms: now,
        policy_id: policy.id,
        policy_kind: policy.policy_kind,
        pressure_behavior: policy.pressure_behavior.clone(),
        raw_history_prune_behavior: policy.raw_history_prune_behavior.clone(),
        sessions_scanned,
        scan_mode: "warn_only".to_string(),
        maintenance_deletes_raw_history: false,
        action_taken: retention_action_taken(
            &policy.pressure_behavior,
            &policy.raw_history_prune_behavior,
        ),
    })
}

struct RetentionPolicyDiagnosticRow {
    id: String,
    policy_kind: String,
    pressure_behavior: String,
    raw_history_prune_behavior: String,
}

fn load_retention_policy(
    connection: &mut SqliteConnection,
    policy_id: &str,
) -> Result<RetentionPolicyDiagnosticRow, TerminalPersistenceV2Error> {
    let (id, policy_kind, pressure_behavior, raw_history_prune_behavior) =
        terminal_retention_policies::table
            .filter(terminal_retention_policies::id.eq(policy_id))
            .select((
                terminal_retention_policies::id,
                terminal_retention_policies::policy_kind,
                terminal_retention_policies::pressure_behavior,
                terminal_retention_policies::raw_history_prune_behavior,
            ))
            .first::<(String, String, String, String)>(connection)?;

    Ok(RetentionPolicyDiagnosticRow {
        id,
        policy_kind,
        pressure_behavior,
        raw_history_prune_behavior,
    })
}

fn retention_action_taken(pressure_behavior: &str, raw_history_prune_behavior: &str) -> String {
    match (pressure_behavior, raw_history_prune_behavior) {
        ("warn_only", "never_silent") => "warn_only_no_delete",
        (_, "request_only") => "warn_only_delete_request_required",
        _ => "warn_only_no_silent_delete",
    }
    .to_string()
}
