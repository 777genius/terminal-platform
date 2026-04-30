use serde::{Deserialize, Serialize};

use terminal_backend_api::{
    BackendCapabilities, BackendSessionSummary, DiscoveredSession, MuxCommandResult,
    ShellLaunchSpec,
};
use terminal_domain::{
    BackendKind, PaneId, SavedSessionCompatibility, SavedSessionManifest, SessionId, SessionRoute,
    SubscriptionId,
};
use terminal_projection::{ScreenDelta, ScreenSnapshot, SessionHealthSnapshot, TopologySnapshot};

use crate::Handshake;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListSessionsResponse {
    pub sessions: Vec<BackendSessionSummary>,
}

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
#[serde(rename_all = "snake_case")]
pub enum PaneHistoryReplayStrategy {
    Empty,
    RawVtStream,
    RenderedSnapshot,
    Mixed,
    Degraded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaneHistoryRestoreEvidence {
    pub kind: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaneHistoryRestorePlan {
    pub session_id: SessionId,
    pub restore_guarantee_level: String,
    pub latest_screen_snapshot_id: Option<String>,
    pub latest_topology_snapshot_id: Option<String>,
    pub high_water_commit_seq: i64,
    pub latest_restore_drill_status: Option<String>,
    pub evidence: Vec<PaneHistoryRestoreEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaneHistoryScreenSnapshot {
    pub id: String,
    pub pane_id: PaneId,
    pub projection_source: String,
    pub buffer_kind: String,
    pub rows: i32,
    pub cols: i32,
    pub base_event_seq: i64,
    pub high_water_event_seq: i64,
    pub high_water_byte_seq: Option<i64>,
    pub screen_json: String,
    pub parser_version: String,
    pub projection_version: String,
    pub checksum: String,
    pub created_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaneHistorySegment {
    pub id: String,
    pub event_seq_low: i64,
    pub event_seq_high: i64,
    pub byte_low: i64,
    pub byte_high: i64,
    pub payload: Vec<u8>,
    pub checksum: String,
    pub capture_semantics: String,
    pub created_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaneHistoryGap {
    pub id: String,
    pub pane_id: Option<PaneId>,
    pub stream_id: String,
    pub gap_kind: String,
    pub event_seq_low: Option<i64>,
    pub event_seq_high: Option<i64>,
    pub byte_low: Option<i64>,
    pub byte_high: Option<i64>,
    pub estimated_dropped_bytes: Option<i64>,
    pub estimated_dropped_events: Option<i64>,
    pub reason: String,
    pub opened_at_ms: i64,
    pub closed_at_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaneHistoryResponse {
    pub session_id: SessionId,
    pub pane_id: PaneId,
    pub from_event_seq: i64,
    pub max_segments: i64,
    pub max_bytes: i64,
    pub restore_plan: PaneHistoryRestorePlan,
    pub latest_screen_snapshot: Option<PaneHistoryScreenSnapshot>,
    pub segments: Vec<PaneHistorySegment>,
    pub gaps: Vec<PaneHistoryGap>,
    pub replay_strategy: PaneHistoryReplayStrategy,
    pub has_more_segments: bool,
    pub next_event_seq: Option<i64>,
    pub total_payload_bytes: i64,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateSessionResponse {
    pub session: BackendSessionSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoverSessionsResponse {
    pub sessions: Vec<DiscoveredSession>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackendCapabilitiesResponse {
    pub backend: BackendKind,
    pub capabilities: BackendCapabilities,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportSessionResponse {
    pub session: BackendSessionSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenSubscriptionResponse {
    pub subscription_id: SubscriptionId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ResponsePayload {
    Handshake(Handshake),
    CreateSession(CreateSessionResponse),
    ListSessions(ListSessionsResponse),
    ListSavedSessions(ListSavedSessionsResponse),
    DiscoverSessions(DiscoverSessionsResponse),
    BackendCapabilities(BackendCapabilitiesResponse),
    ImportSession(ImportSessionResponse),
    SavedSession(SavedSessionResponse),
    DeleteSavedSession(DeleteSavedSessionResponse),
    PruneSavedSessions(PruneSavedSessionsResponse),
    RestoreSavedSession(RestoreSavedSessionResponse),
    TopologySnapshot(TopologySnapshot),
    SessionHealthSnapshot(SessionHealthSnapshot),
    ScreenSnapshot(ScreenSnapshot),
    ScreenDelta(ScreenDelta),
    PaneHistory(PaneHistoryResponse),
    CommandHistory(CommandHistoryResponse),
    DispatchMuxCommand(MuxCommandResult),
    SubscriptionOpened(OpenSubscriptionResponse),
}
