use super::{prelude::*, *};

impl From<&SavedSessionManifest> for NodeSavedSessionManifest {
    fn from(value: &SavedSessionManifest) -> Self {
        Self {
            format_version: value.format_version,
            binary_version: value.binary_version.clone(),
            protocol_major: value.protocol_major,
            protocol_minor: value.protocol_minor,
        }
    }
}

impl From<&SavedSessionCompatibilityStatus> for NodeSavedSessionCompatibilityStatus {
    fn from(value: &SavedSessionCompatibilityStatus) -> Self {
        match value {
            SavedSessionCompatibilityStatus::Compatible => Self::Compatible,
            SavedSessionCompatibilityStatus::BinarySkew => Self::BinarySkew,
            SavedSessionCompatibilityStatus::FormatVersionUnsupported => {
                Self::FormatVersionUnsupported
            }
            SavedSessionCompatibilityStatus::ProtocolMajorUnsupported => {
                Self::ProtocolMajorUnsupported
            }
            SavedSessionCompatibilityStatus::ProtocolMinorAhead => Self::ProtocolMinorAhead,
        }
    }
}

impl From<&SavedSessionCompatibility> for NodeSavedSessionCompatibility {
    fn from(value: &SavedSessionCompatibility) -> Self {
        Self { can_restore: value.can_restore, status: (&value.status).into() }
    }
}

impl From<&SavedSessionRestoreSemantics> for NodeSavedSessionRestoreSemantics {
    fn from(value: &SavedSessionRestoreSemantics) -> Self {
        Self {
            restores_topology: value.restores_topology,
            restores_focus_state: value.restores_focus_state,
            restores_tab_titles: value.restores_tab_titles,
            uses_saved_launch_spec: value.uses_saved_launch_spec,
            replays_saved_screen_buffers: value.replays_saved_screen_buffers,
            preserves_process_state: value.preserves_process_state,
        }
    }
}

impl From<&RestoreGuaranteeLevel> for NodeRestoreGuaranteeLevel {
    fn from(value: &RestoreGuaranteeLevel) -> Self {
        match value {
            RestoreGuaranteeLevel::RichHistory => Self::RichHistory,
            RestoreGuaranteeLevel::BasicHistory => Self::BasicHistory,
            RestoreGuaranteeLevel::VisualRestoreOnly => Self::VisualRestoreOnly,
            RestoreGuaranteeLevel::HistoryDegraded => Self::HistoryDegraded,
        }
    }
}

impl From<&HistoryReplayState> for NodeHistoryReplayState {
    fn from(value: &HistoryReplayState) -> Self {
        match value {
            HistoryReplayState::NotAvailable => Self::NotAvailable,
            HistoryReplayState::SnapshotOnly => Self::SnapshotOnly,
            HistoryReplayState::HydratedFromSnapshot => Self::HydratedFromSnapshot,
            HistoryReplayState::ReplayedFromJournal => Self::ReplayedFromJournal,
            HistoryReplayState::PartiallyReplayedWithGaps => Self::PartiallyReplayedWithGaps,
        }
    }
}

impl From<&SavedSessionRestoreSemanticsV2> for NodeSavedSessionRestoreSemanticsV2 {
    fn from(value: &SavedSessionRestoreSemanticsV2) -> Self {
        Self {
            restores_topology: value.restores_topology,
            restores_focus_state: value.restores_focus_state,
            restores_tab_titles: value.restores_tab_titles,
            uses_saved_launch_spec: value.uses_saved_launch_spec,
            replays_saved_screen_buffers: value.replays_saved_screen_buffers,
            preserves_process_state: value.preserves_process_state,
            restore_guarantee_level: (&value.restore_guarantee_level).into(),
            history_replay_state: (&value.history_replay_state).into(),
            source_session_id: value.source_session_id.0.to_string(),
            restored_session_id: value
                .restored_session_id
                .as_ref()
                .map(|session_id| session_id.0.to_string()),
            latest_restore_drill_status: value.latest_restore_drill_status.clone(),
            has_known_gaps: value.has_known_gaps,
            evidence_refs: value.evidence_refs.clone(),
        }
    }
}

impl From<&SavedSessionSummary> for NodeSavedSessionSummary {
    fn from(value: &SavedSessionSummary) -> Self {
        Self {
            session_id: value.session_id.0.to_string(),
            route: (&value.route).into(),
            title: value.title.clone(),
            saved_at_ms: value.saved_at_ms,
            manifest: (&value.manifest).into(),
            compatibility: (&value.compatibility).into(),
            has_launch: value.has_launch,
            tab_count: value.tab_count,
            pane_count: value.pane_count,
            restore_semantics: (&value.restore_semantics).into(),
            restore_semantics_v2: value.restore_semantics_v2.as_ref().map(Into::into),
        }
    }
}

impl From<&SavedSessionRecord> for NodeSavedSessionRecord {
    fn from(value: &SavedSessionRecord) -> Self {
        Self {
            session_id: value.session_id.0.to_string(),
            route: (&value.route).into(),
            title: value.title.clone(),
            launch: value.launch.as_ref().map(Into::into),
            manifest: (&value.manifest).into(),
            compatibility: (&value.compatibility).into(),
            topology: (&value.topology).into(),
            screens: value.screens.iter().map(Into::into).collect(),
            saved_at_ms: value.saved_at_ms,
            restore_semantics: (&value.restore_semantics).into(),
            restore_semantics_v2: value.restore_semantics_v2.as_ref().map(Into::into),
        }
    }
}

impl From<&RestoreSavedSessionResponse> for NodeRestoredSession {
    fn from(value: &RestoreSavedSessionResponse) -> Self {
        Self {
            saved_session_id: value.saved_session_id.0.to_string(),
            manifest: (&value.manifest).into(),
            compatibility: (&value.compatibility).into(),
            session: (&value.session).into(),
            restore_semantics: (&value.restore_semantics).into(),
            restore_semantics_v2: value.restore_semantics_v2.as_ref().map(Into::into),
        }
    }
}

impl From<&DeleteSavedSessionResponse> for NodeDeleteSavedSessionResult {
    fn from(value: &DeleteSavedSessionResponse) -> Self {
        Self { session_id: value.session_id.0.to_string() }
    }
}

impl From<&PruneSavedSessionsResponse> for NodePruneSavedSessionsResult {
    fn from(value: &PruneSavedSessionsResponse) -> Self {
        Self { deleted_count: value.deleted_count, kept_count: value.kept_count }
    }
}
