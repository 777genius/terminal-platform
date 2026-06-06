use super::super::super::*;
use super::metrics::RestorePlanInputs;

pub(super) fn choose_restore_guarantee(
    inputs: &RestorePlanInputs,
    now: i64,
) -> RestoreGuaranteeLevel {
    let has_fresh_raw_capability = inputs.latest_capability_report.as_ref().is_some_and(|report| {
        !capability_report_is_stale(report, now) && report.capture_semantics == "raw_vt_stream"
    });

    let mut guarantee_level = match (
        inputs.segment_count > 0,
        inputs.raw_segment_count > 0,
        inputs.latest_screen.is_some(),
        inputs.latest_topology.is_some(),
        inputs.gap_count > 0,
    ) {
        (_, _, _, _, true) => RestoreGuaranteeLevel::DegradedHistory,
        (true, true, true, true, false)
            if inputs.latest_restore_drill_status.as_deref() == Some("passed")
                && has_fresh_raw_capability =>
        {
            RestoreGuaranteeLevel::RawStreamReplay
        }
        (true, _, true, _, false) => RestoreGuaranteeLevel::BasicHistory,
        (false, _, true, _, false) => RestoreGuaranteeLevel::VisualSnapshotOnly,
        _ => RestoreGuaranteeLevel::None,
    };

    downgrade_for_global_guards(&mut guarantee_level, inputs);
    downgrade_for_capability(&mut guarantee_level, inputs, now);
    guarantee_level
}

pub(super) fn capability_report_is_stale(report: &BackendCapabilityReportRow, now: i64) -> bool {
    report.expires_at_ms <= now || report.stale_reason.is_some() || report.probe_status != "passed"
}

fn downgrade_for_global_guards(
    guarantee_level: &mut RestoreGuaranteeLevel,
    inputs: &RestorePlanInputs,
) {
    if matches!(inputs.latest_restore_drill_status.as_deref(), Some("failed" | "degraded")) {
        *guarantee_level = RestoreGuaranteeLevel::DegradedHistory;
    }
    if inputs.authoritative_reads_gate == FeatureGateState::ForceDisabled.as_str() {
        *guarantee_level = RestoreGuaranteeLevel::DegradedHistory;
    }
    if inputs.critical_health_record_count > 0 {
        *guarantee_level = RestoreGuaranteeLevel::DegradedHistory;
    }
}

fn downgrade_for_capability(
    guarantee_level: &mut RestoreGuaranteeLevel,
    inputs: &RestorePlanInputs,
    now: i64,
) {
    if let Some(report) = inputs.latest_capability_report.as_ref() {
        if capability_report_is_stale(report, now) {
            *guarantee_level = RestoreGuaranteeLevel::DegradedHistory;
        }
        if report.capture_semantics != "raw_vt_stream"
            && matches!(guarantee_level, RestoreGuaranteeLevel::RawStreamReplay)
        {
            *guarantee_level = RestoreGuaranteeLevel::BasicHistory;
        }
    }
}
