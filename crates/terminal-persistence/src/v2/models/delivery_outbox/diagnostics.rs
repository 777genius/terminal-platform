use super::super::super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutboxDiagnosticsRecord {
    pub generated_at_ms: i64,
    pub pending_count: i64,
    pub due_pending_count: i64,
    pub claimed_count: i64,
    pub stale_claim_count: i64,
    pub done_count: i64,
    pub failed_count: i64,
    pub quarantined_count: i64,
    pub oldest_due_pending_age_ms: Option<i64>,
    pub next_pending_due_in_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompressionDiagnosticsRecord {
    pub generated_at_ms: i64,
    pub feature_gate_state: String,
    pub raw_segment_count: i64,
    pub zstd_segment_count: i64,
    pub unsupported_segment_count: i64,
    pub rewrite_candidate_count: i64,
    pub segments_rewritten: i64,
    pub restore_drill_required: bool,
    pub action_taken: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetentionDiagnosticsRecord {
    pub generated_at_ms: i64,
    pub policy_id: String,
    pub policy_kind: String,
    pub pressure_behavior: String,
    pub raw_history_prune_behavior: String,
    pub sessions_scanned: i64,
    pub scan_mode: String,
    pub maintenance_deletes_raw_history: bool,
    pub action_taken: String,
}
