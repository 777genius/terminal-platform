use super::{prelude::*, *};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum NodeSavedSessionCompatibilityStatus {
    Compatible,
    BinarySkew,
    FormatVersionUnsupported,
    ProtocolMajorUnsupported,
    ProtocolMinorAhead,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeSavedSessionManifest {
    pub format_version: u32,
    pub binary_version: String,
    pub protocol_major: u16,
    pub protocol_minor: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeSavedSessionCompatibility {
    pub can_restore: bool,
    pub status: NodeSavedSessionCompatibilityStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeSavedSessionRestoreSemantics {
    pub restores_topology: bool,
    pub restores_focus_state: bool,
    pub restores_tab_titles: bool,
    pub uses_saved_launch_spec: bool,
    pub replays_saved_screen_buffers: bool,
    pub preserves_process_state: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum NodeRestoreGuaranteeLevel {
    RichHistory,
    BasicHistory,
    VisualRestoreOnly,
    HistoryDegraded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum NodeHistoryReplayState {
    NotAvailable,
    SnapshotOnly,
    HydratedFromSnapshot,
    ReplayedFromJournal,
    PartiallyReplayedWithGaps,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeSavedSessionRestoreSemanticsV2 {
    pub restores_topology: bool,
    pub restores_focus_state: bool,
    pub restores_tab_titles: bool,
    pub uses_saved_launch_spec: bool,
    pub replays_saved_screen_buffers: bool,
    pub preserves_process_state: bool,
    pub restore_guarantee_level: NodeRestoreGuaranteeLevel,
    pub history_replay_state: NodeHistoryReplayState,
    pub source_session_id: String,
    pub restored_session_id: Option<String>,
    pub latest_restore_drill_status: Option<String>,
    pub has_known_gaps: bool,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeSavedSessionSummary {
    pub session_id: String,
    pub route: NodeSessionRoute,
    pub title: Option<String>,
    pub saved_at_ms: i64,
    pub manifest: NodeSavedSessionManifest,
    pub compatibility: NodeSavedSessionCompatibility,
    pub has_launch: bool,
    pub tab_count: usize,
    pub pane_count: usize,
    pub restore_semantics: NodeSavedSessionRestoreSemantics,
    pub restore_semantics_v2: Option<NodeSavedSessionRestoreSemanticsV2>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeSavedSessionRecord {
    pub session_id: String,
    pub route: NodeSessionRoute,
    pub title: Option<String>,
    pub launch: Option<NodeShellLaunchSpec>,
    pub manifest: NodeSavedSessionManifest,
    pub compatibility: NodeSavedSessionCompatibility,
    pub topology: NodeTopologySnapshot,
    pub screens: Vec<NodeScreenSnapshot>,
    pub saved_at_ms: i64,
    pub restore_semantics: NodeSavedSessionRestoreSemantics,
    pub restore_semantics_v2: Option<NodeSavedSessionRestoreSemanticsV2>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeRestoredSession {
    pub saved_session_id: String,
    pub manifest: NodeSavedSessionManifest,
    pub compatibility: NodeSavedSessionCompatibility,
    pub session: NodeSessionSummary,
    pub restore_semantics: NodeSavedSessionRestoreSemantics,
    pub restore_semantics_v2: Option<NodeSavedSessionRestoreSemanticsV2>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeDeleteSavedSessionResult {
    pub session_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodePruneSavedSessionsResult {
    pub deleted_count: usize,
    pub kept_count: usize,
}
