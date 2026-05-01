#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::v2) struct MaintenanceRecoverySummary {
    pub(in crate::v2) stale_outbox_claims_requeued: usize,
    pub(in crate::v2) stale_outbox_claims_quarantined: usize,
    pub(in crate::v2) stale_writer_generations_marked: usize,
}
