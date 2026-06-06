use terminal_backend_api::ShellLaunchSpec;
use terminal_domain::{SavedSessionManifest, SessionId, SessionRoute};
use terminal_persistence::RestorePlan;
use terminal_projection::{ScreenSnapshot, TopologySnapshot};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSavedSessionRecord {
    pub session_id: SessionId,
    pub route: SessionRoute,
    pub title: Option<String>,
    pub launch: Option<ShellLaunchSpec>,
    pub manifest: SavedSessionManifest,
    pub topology: TopologySnapshot,
    pub screens: Vec<ScreenSnapshot>,
    pub saved_at_ms: i64,
    pub restore_plan: Option<RestorePlan>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSavedSessionSummary {
    pub session_id: SessionId,
    pub route: SessionRoute,
    pub title: Option<String>,
    pub saved_at_ms: i64,
    pub manifest: SavedSessionManifest,
    pub has_launch: bool,
    pub tab_count: usize,
    pub pane_count: usize,
    pub restore_plan: Option<RestorePlan>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePrunedSavedSessions {
    pub deleted_count: usize,
    pub kept_count: usize,
}
