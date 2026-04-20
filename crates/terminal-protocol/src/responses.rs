use serde::{Deserialize, Serialize};

use terminal_backend_api::{
    BackendCapabilities, BackendSessionSummary, DiscoveredSession, MuxCommandResult,
    ShellLaunchSpec,
};
use terminal_domain::{
    BackendKind, SavedSessionCompatibility, SavedSessionManifest, SessionId, SessionRoute,
    SubscriptionId,
};
use terminal_projection::{ScreenDelta, ScreenSnapshot, TopologySnapshot};

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
    ScreenSnapshot(ScreenSnapshot),
    ScreenDelta(ScreenDelta),
    DispatchMuxCommand(MuxCommandResult),
    SubscriptionOpened(OpenSubscriptionResponse),
}
