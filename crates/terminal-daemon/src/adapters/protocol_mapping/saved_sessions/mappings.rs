use terminal_persistence::{
    RestoreGuaranteeLevel as PersistenceRestoreGuaranteeLevel, RestorePlan,
};
use terminal_protocol::{HistoryReplayState, RestoreGuaranteeLevel};

pub(super) fn map_restore_guarantee_level(
    value: &PersistenceRestoreGuaranteeLevel,
) -> RestoreGuaranteeLevel {
    match value {
        PersistenceRestoreGuaranteeLevel::RawStreamReplay => RestoreGuaranteeLevel::RichHistory,
        PersistenceRestoreGuaranteeLevel::BasicHistory => RestoreGuaranteeLevel::BasicHistory,
        PersistenceRestoreGuaranteeLevel::VisualSnapshotOnly => {
            RestoreGuaranteeLevel::VisualRestoreOnly
        }
        PersistenceRestoreGuaranteeLevel::LiveMuxAttach => RestoreGuaranteeLevel::BasicHistory,
        PersistenceRestoreGuaranteeLevel::DegradedHistory
        | PersistenceRestoreGuaranteeLevel::None => RestoreGuaranteeLevel::HistoryDegraded,
    }
}

pub(super) fn map_history_replay_state(
    restore_plan: &RestorePlan,
    has_known_gaps: bool,
) -> HistoryReplayState {
    if has_known_gaps {
        return HistoryReplayState::PartiallyReplayedWithGaps;
    }
    match restore_plan.guarantee_level {
        PersistenceRestoreGuaranteeLevel::RawStreamReplay => {
            HistoryReplayState::ReplayedFromJournal
        }
        PersistenceRestoreGuaranteeLevel::BasicHistory => HistoryReplayState::ReplayedFromJournal,
        PersistenceRestoreGuaranteeLevel::VisualSnapshotOnly => {
            HistoryReplayState::HydratedFromSnapshot
        }
        PersistenceRestoreGuaranteeLevel::LiveMuxAttach => HistoryReplayState::HydratedFromSnapshot,
        PersistenceRestoreGuaranteeLevel::None => HistoryReplayState::NotAvailable,
        PersistenceRestoreGuaranteeLevel::DegradedHistory => {
            if restore_plan.latest_screen_snapshot_id.is_some() {
                HistoryReplayState::SnapshotOnly
            } else {
                HistoryReplayState::NotAvailable
            }
        }
    }
}
