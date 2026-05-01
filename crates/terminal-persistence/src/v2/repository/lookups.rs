use super::super::*;

pub(in crate::v2) fn session_private_mode(
    connection: &mut SqliteConnection,
    session_id: &str,
) -> Result<bool, TerminalPersistenceV2Error> {
    terminal_sessions::table
        .filter(terminal_sessions::id.eq(session_id))
        .select(terminal_sessions::private_mode)
        .first::<i32>(connection)
        .optional()
        .map(|value| value.unwrap_or(0) != 0)
        .map_err(Into::into)
}

pub(in crate::v2) fn latest_backend_capability_report(
    connection: &mut SqliteConnection,
    session_id: &str,
) -> Result<Option<BackendCapabilityReportRow>, TerminalPersistenceV2Error> {
    terminal_backend_capability_reports::table
        .filter(terminal_backend_capability_reports::session_id.eq(Some(session_id.to_string())))
        .order(terminal_backend_capability_reports::created_at_ms.desc())
        .select(BackendCapabilityReportRow::as_select())
        .first::<BackendCapabilityReportRow>(connection)
        .optional()
        .map_err(Into::into)
}

pub(in crate::v2) fn load_maintenance_run(
    connection: &mut SqliteConnection,
    id: &str,
) -> Result<MaintenanceRunRow, TerminalPersistenceV2Error> {
    terminal_maintenance_runs::table
        .filter(terminal_maintenance_runs::id.eq(id))
        .select(MaintenanceRunRow::as_select())
        .first::<MaintenanceRunRow>(connection)
        .map_err(Into::into)
}

pub(in crate::v2) fn load_export_request(
    connection: &mut SqliteConnection,
    id: &str,
) -> Result<ExportRequestRow, TerminalPersistenceV2Error> {
    terminal_export_requests::table
        .filter(terminal_export_requests::id.eq(id))
        .select(ExportRequestRow::as_select())
        .first::<ExportRequestRow>(connection)
        .map_err(Into::into)
}

pub(in crate::v2) fn load_support_bundle(
    connection: &mut SqliteConnection,
    id: &str,
) -> Result<SupportBundleRow, TerminalPersistenceV2Error> {
    terminal_support_bundles::table
        .filter(terminal_support_bundles::id.eq(id))
        .select(SupportBundleRow::as_select())
        .first::<SupportBundleRow>(connection)
        .map_err(Into::into)
}

pub(in crate::v2) fn load_ai_context_package(
    connection: &mut SqliteConnection,
    id: &str,
) -> Result<AiContextPackageRow, TerminalPersistenceV2Error> {
    terminal_ai_context_packages::table
        .filter(terminal_ai_context_packages::id.eq(id))
        .select(AiContextPackageRow::as_select())
        .first::<AiContextPackageRow>(connection)
        .map_err(Into::into)
}

pub(in crate::v2) fn load_ai_action_approval(
    connection: &mut SqliteConnection,
    id: &str,
) -> Result<AiActionApprovalRow, TerminalPersistenceV2Error> {
    terminal_ai_action_approvals::table
        .filter(terminal_ai_action_approvals::id.eq(id))
        .select(AiActionApprovalRow::as_select())
        .first::<AiActionApprovalRow>(connection)
        .map_err(Into::into)
}
