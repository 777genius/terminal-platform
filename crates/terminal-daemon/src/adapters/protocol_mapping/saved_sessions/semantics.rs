use terminal_domain::SessionId;
use terminal_persistence::{
    RestoreGuaranteeLevel as PersistenceRestoreGuaranteeLevel, RestorePlan,
};
use terminal_protocol::{SavedSessionRestoreSemantics, SavedSessionRestoreSemanticsV2};

use super::mappings::{map_history_replay_state, map_restore_guarantee_level};

pub(super) fn saved_session_restore_semantics(
    has_launch: bool,
    restore_plan: Option<&RestorePlan>,
) -> SavedSessionRestoreSemantics {
    SavedSessionRestoreSemantics {
        restores_topology: true,
        restores_focus_state: true,
        restores_tab_titles: true,
        uses_saved_launch_spec: has_launch,
        replays_saved_screen_buffers: restore_plan_proves_screen_replay(restore_plan),
        preserves_process_state: false,
    }
}

pub(super) fn saved_session_restore_semantics_v2(
    source_session_id: SessionId,
    restored_session_id: Option<SessionId>,
    legacy: &SavedSessionRestoreSemantics,
    restore_plan: Option<&RestorePlan>,
) -> Option<SavedSessionRestoreSemanticsV2> {
    let restore_plan = restore_plan?;
    let has_known_gaps = restore_plan.evidence.iter().any(|evidence| {
        evidence.kind == "history_gap"
            || evidence.kind == "history_gap_count"
                && evidence.value.parse::<i64>().unwrap_or(0) > 0
    });
    Some(SavedSessionRestoreSemanticsV2 {
        restores_topology: legacy.restores_topology,
        restores_focus_state: legacy.restores_focus_state,
        restores_tab_titles: legacy.restores_tab_titles,
        uses_saved_launch_spec: legacy.uses_saved_launch_spec,
        replays_saved_screen_buffers: restore_plan_proves_screen_replay(Some(restore_plan)),
        preserves_process_state: legacy.preserves_process_state,
        restore_guarantee_level: map_restore_guarantee_level(&restore_plan.guarantee_level),
        history_replay_state: map_history_replay_state(restore_plan, has_known_gaps),
        source_session_id,
        restored_session_id,
        latest_restore_drill_status: restore_plan.latest_restore_drill_status.clone(),
        has_known_gaps,
        evidence_refs: restore_plan
            .evidence
            .iter()
            .map(|evidence| format!("{}:{}", evidence.kind, evidence.value))
            .collect(),
    })
}

fn restore_plan_proves_screen_replay(restore_plan: Option<&RestorePlan>) -> bool {
    let Some(restore_plan) = restore_plan else {
        return false;
    };
    let has_passing_drill = restore_plan.latest_restore_drill_status.as_deref() == Some("passed");
    let replayable_guarantee = matches!(
        restore_plan.guarantee_level,
        PersistenceRestoreGuaranteeLevel::RawStreamReplay
            | PersistenceRestoreGuaranteeLevel::BasicHistory
    );
    has_passing_drill && replayable_guarantee
}

#[cfg(test)]
mod tests {
    use terminal_domain::SessionId;
    use terminal_persistence::{
        RestoreEvidence, RestoreGuaranteeLevel as PersistenceRestoreGuaranteeLevel, RestorePlan,
    };
    use terminal_protocol::{HistoryReplayState, RestoreGuaranteeLevel};
    use uuid::Uuid;

    use super::{
        restore_plan_proves_screen_replay, saved_session_restore_semantics,
        saved_session_restore_semantics_v2,
    };

    fn restore_plan(
        guarantee_level: PersistenceRestoreGuaranteeLevel,
        latest_restore_drill_status: Option<&str>,
    ) -> RestorePlan {
        RestorePlan {
            session_id: Uuid::new_v4().to_string(),
            guarantee_level,
            latest_screen_snapshot_id: Some("screen-snapshot".to_string()),
            latest_topology_snapshot_id: Some("topology-snapshot".to_string()),
            high_water_commit_seq: 1,
            latest_restore_drill_status: latest_restore_drill_status.map(str::to_string),
            evidence: vec![RestoreEvidence {
                kind: "stream_segment_count".to_string(),
                value: "1".to_string(),
            }],
        }
    }

    #[test]
    fn legacy_snapshot_only_semantics_do_not_claim_screen_replay() {
        let source_session_id = SessionId::new();
        let plan =
            restore_plan(PersistenceRestoreGuaranteeLevel::VisualSnapshotOnly, Some("passed"));
        let legacy = saved_session_restore_semantics(true, Some(&plan));
        let v2 = saved_session_restore_semantics_v2(source_session_id, None, &legacy, Some(&plan))
            .expect("v2 semantics should map");

        assert!(!legacy.replays_saved_screen_buffers);
        assert!(!v2.replays_saved_screen_buffers);
        assert_eq!(v2.restore_guarantee_level, RestoreGuaranteeLevel::VisualRestoreOnly);
        assert_eq!(v2.history_replay_state, HistoryReplayState::HydratedFromSnapshot);
    }

    #[test]
    fn v2_basic_history_with_restore_drill_claims_screen_replay() {
        let source_session_id = SessionId::new();
        let plan = restore_plan(PersistenceRestoreGuaranteeLevel::BasicHistory, Some("passed"));
        let legacy = saved_session_restore_semantics(true, Some(&plan));
        let v2 = saved_session_restore_semantics_v2(source_session_id, None, &legacy, Some(&plan))
            .expect("v2 semantics should map");

        assert!(legacy.replays_saved_screen_buffers);
        assert!(v2.replays_saved_screen_buffers);
        assert_eq!(v2.restore_guarantee_level, RestoreGuaranteeLevel::BasicHistory);
        assert_eq!(v2.history_replay_state, HistoryReplayState::ReplayedFromJournal);
    }

    #[test]
    fn v2_basic_history_without_restore_drill_stays_conservative() {
        let plan = restore_plan(PersistenceRestoreGuaranteeLevel::BasicHistory, None);
        let legacy = saved_session_restore_semantics(true, Some(&plan));

        assert!(!legacy.replays_saved_screen_buffers);
        assert!(!restore_plan_proves_screen_replay(Some(&plan)));
    }
}
