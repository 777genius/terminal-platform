use serde::{Deserialize, Serialize};

use terminal_backend_api::{BackendSessionSummary, ShellLaunchSpec};
use terminal_domain::{
    PaneId, SavedSessionCompatibility, SavedSessionManifest, SessionId, SessionRoute,
};
use terminal_projection::{ScreenSnapshot, TopologySnapshot};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedSessionSummary {
    pub session_id: SessionId,
    pub route: SessionRoute,
    pub title: Option<String>,
    pub saved_at_ms: i64,
    pub manifest: SavedSessionManifest,
    pub compatibility: SavedSessionCompatibility,
    pub has_launch: bool,
    pub tab_count: usize,
    pub pane_count: usize,
    pub restore_semantics: SavedSessionRestoreSemantics,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restore_semantics_v2: Option<SavedSessionRestoreSemanticsV2>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedSessionRestoreSemantics {
    pub restores_topology: bool,
    pub restores_focus_state: bool,
    pub restores_tab_titles: bool,
    pub uses_saved_launch_spec: bool,
    pub replays_saved_screen_buffers: bool,
    pub preserves_process_state: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RestoreGuaranteeLevel {
    RichHistory,
    BasicHistory,
    VisualRestoreOnly,
    HistoryDegraded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HistoryReplayState {
    NotAvailable,
    SnapshotOnly,
    HydratedFromSnapshot,
    ReplayedFromJournal,
    PartiallyReplayedWithGaps,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedSessionRestoreSemanticsV2 {
    pub restores_topology: bool,
    pub restores_focus_state: bool,
    pub restores_tab_titles: bool,
    pub uses_saved_launch_spec: bool,
    pub replays_saved_screen_buffers: bool,
    pub preserves_process_state: bool,
    pub restore_guarantee_level: RestoreGuaranteeLevel,
    pub history_replay_state: HistoryReplayState,
    pub source_session_id: SessionId,
    pub restored_session_id: Option<SessionId>,
    pub latest_restore_drill_status: Option<String>,
    pub has_known_gaps: bool,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedSessionRecord {
    pub session_id: SessionId,
    pub route: SessionRoute,
    pub title: Option<String>,
    pub launch: Option<ShellLaunchSpec>,
    pub manifest: SavedSessionManifest,
    pub compatibility: SavedSessionCompatibility,
    pub topology: TopologySnapshot,
    pub screens: Vec<ScreenSnapshot>,
    pub saved_at_ms: i64,
    pub restore_semantics: SavedSessionRestoreSemantics,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restore_semantics_v2: Option<SavedSessionRestoreSemanticsV2>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListSavedSessionsResponse {
    pub sessions: Vec<SavedSessionSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedSessionResponse {
    pub session: SavedSessionRecord,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestoreSavedSessionResponse {
    pub saved_session_id: SessionId,
    pub manifest: SavedSessionManifest,
    pub compatibility: SavedSessionCompatibility,
    pub session: BackendSessionSummary,
    pub restore_semantics: SavedSessionRestoreSemantics,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restore_semantics_v2: Option<SavedSessionRestoreSemanticsV2>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeleteSavedSessionResponse {
    pub session_id: SessionId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PruneSavedSessionsResponse {
    pub deleted_count: usize,
    pub kept_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandHistoryEntry {
    pub id: String,
    pub session_id: Option<SessionId>,
    pub pane_id: Option<PaneId>,
    pub display_text: String,
    pub last_used_at_ms: i64,
    pub use_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandHistoryResponse {
    pub entries: Vec<CommandHistoryEntry>,
}
