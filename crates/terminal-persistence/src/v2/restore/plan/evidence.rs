use super::super::super::*;
use super::guarantee::capability_report_is_stale;
use super::metrics::RestorePlanInputs;

pub(super) fn build_restore_evidence(
    session_id: &str,
    inputs: &RestorePlanInputs,
    now: i64,
) -> Vec<RestoreEvidence> {
    let mut evidence = base_restore_evidence(inputs);
    append_snapshot_evidence(&mut evidence, inputs);
    append_restore_drill_evidence(&mut evidence, inputs);
    append_capability_evidence(&mut evidence, inputs, now);
    if let (Some(event_seq_low), Some(event_seq_high)) = inputs.stream_event_range {
        evidence.push(RestoreEvidence {
            kind: "journal_event_range".to_string(),
            value: format!("{session_id}:{event_seq_low}:{event_seq_high}"),
        });
    }
    evidence
}

fn base_restore_evidence(inputs: &RestorePlanInputs) -> Vec<RestoreEvidence> {
    vec![
        RestoreEvidence {
            kind: "stream_segment_count".to_string(),
            value: inputs.segment_count.to_string(),
        },
        RestoreEvidence {
            kind: "raw_stream_segment_count".to_string(),
            value: inputs.raw_segment_count.to_string(),
        },
        RestoreEvidence {
            kind: "rendered_stream_segment_count".to_string(),
            value: inputs.rendered_segment_count.to_string(),
        },
        RestoreEvidence {
            kind: "history_gap_count".to_string(),
            value: inputs.gap_count.to_string(),
        },
        RestoreEvidence {
            kind: "authoritative_reads_gate_state".to_string(),
            value: inputs.authoritative_reads_gate.clone(),
        },
        RestoreEvidence {
            kind: "critical_data_health_record_count".to_string(),
            value: inputs.critical_health_record_count.to_string(),
        },
    ]
}

fn append_snapshot_evidence(evidence: &mut Vec<RestoreEvidence>, inputs: &RestorePlanInputs) {
    if let Some(screen) = inputs.latest_screen.as_ref() {
        evidence.push(RestoreEvidence {
            kind: "screen_snapshot".to_string(),
            value: screen.id.clone(),
        });
    }
    if let Some(topology) = inputs.latest_topology.as_ref() {
        evidence.push(RestoreEvidence {
            kind: "topology_snapshot".to_string(),
            value: topology.id.clone(),
        });
    }
}

fn append_restore_drill_evidence(evidence: &mut Vec<RestoreEvidence>, inputs: &RestorePlanInputs) {
    if let Some(status) = &inputs.latest_restore_drill_status {
        evidence.push(RestoreEvidence {
            kind: "latest_restore_drill_status".to_string(),
            value: status.clone(),
        });
    }
    if let Some((drill_id, _)) = &inputs.latest_restore_drill {
        evidence
            .push(RestoreEvidence { kind: "restore_drill".to_string(), value: drill_id.clone() });
    }
}

fn append_capability_evidence(
    evidence: &mut Vec<RestoreEvidence>,
    inputs: &RestorePlanInputs,
    now: i64,
) {
    if let Some(report) = inputs.latest_capability_report.as_ref() {
        evidence.push(RestoreEvidence {
            kind: "backend_capability_report".to_string(),
            value: report.id.clone(),
        });
        evidence.push(RestoreEvidence {
            kind: "backend_capability_probe_status".to_string(),
            value: report.probe_status.clone(),
        });
        evidence.push(RestoreEvidence {
            kind: "backend_capture_strategy".to_string(),
            value: report.capture_strategy.clone(),
        });
        evidence.push(RestoreEvidence {
            kind: "backend_capture_semantics".to_string(),
            value: report.capture_semantics.clone(),
        });
        evidence.push(RestoreEvidence {
            kind: "backend_can_preserve_process_when_live".to_string(),
            value: sqlite_bool_evidence(report.can_preserve_process_when_live).to_string(),
        });
        evidence.push(RestoreEvidence {
            kind: "backend_can_capture_scrollback".to_string(),
            value: sqlite_bool_evidence(report.can_capture_scrollback).to_string(),
        });
        evidence.push(RestoreEvidence {
            kind: "backend_capability_stale".to_string(),
            value: capability_report_is_stale(report, now).to_string(),
        });
        if let Some(reason) = report.stale_reason.as_ref() {
            evidence.push(RestoreEvidence {
                kind: "backend_capability_stale_reason".to_string(),
                value: reason.clone(),
            });
        }
    }
}

fn sqlite_bool_evidence(value: i32) -> bool {
    value != 0
}
