use super::super::super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiInputEventInput {
    pub session_id: String,
    pub route: SessionRoute,
    pub title: Option<String>,
    pub launch: Option<ShellLaunchSpec>,
    pub pane_id: String,
    pub data: String,
    pub is_paste: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_event_id: Option<String>,
    pub rows: Option<i32>,
    pub cols: Option<i32>,
    pub shell_kind: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShellMetadataProfile {
    pub shell_kind: Option<String>,
    pub command_boundary_confidence: String,
    pub cwd_source: String,
    pub input_terminator: String,
    pub windows_profile: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalOutputEventInput {
    pub session_id: String,
    pub route: SessionRoute,
    pub title: Option<String>,
    pub launch: Option<ShellLaunchSpec>,
    pub pane_id: String,
    pub tab_id: Option<String>,
    pub payload: Vec<u8>,
    pub rows: Option<i32>,
    pub cols: Option<i32>,
    pub source_sequence: Option<u64>,
    pub occurred_at_ms: Option<i64>,
    pub capture_semantics: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryGapEventInput {
    pub session_id: String,
    pub route: SessionRoute,
    pub title: Option<String>,
    pub launch: Option<ShellLaunchSpec>,
    pub pane_id: String,
    pub tab_id: Option<String>,
    pub rows: Option<i32>,
    pub cols: Option<i32>,
    pub skipped_events: u64,
    pub estimated_dropped_bytes: Option<i64>,
    pub reason: String,
    pub occurred_at_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenSnapshotEventInput {
    pub session_id: String,
    pub route: SessionRoute,
    pub title: Option<String>,
    pub launch: Option<ShellLaunchSpec>,
    pub tab_id: Option<String>,
    pub screen: ScreenSnapshot,
    pub buffer_kind: Option<String>,
    pub capture_semantics: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologySnapshotEventInput {
    pub session_id: String,
    pub route: SessionRoute,
    pub title: Option<String>,
    pub launch: Option<ShellLaunchSpec>,
    pub topology: TopologySnapshot,
}
