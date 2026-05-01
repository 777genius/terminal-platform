use super::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum NodeProjectionSource {
    NativeEmulator,
    NativeTranscript,
    TmuxCapturePane,
    TmuxRawOutputImport,
    ZellijViewportSubscribe,
    ZellijDumpSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeScreenCursor {
    pub row: u16,
    pub col: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeScreenLine {
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeScreenSurface {
    pub title: Option<String>,
    pub cursor: Option<NodeScreenCursor>,
    pub lines: Vec<NodeScreenLine>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeScreenSnapshot {
    pub pane_id: String,
    pub sequence: u64,
    pub rows: u16,
    pub cols: u16,
    pub source: NodeProjectionSource,
    pub surface: NodeScreenSurface,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeScreenLinePatch {
    pub row: u16,
    pub line: NodeScreenLine,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeScreenPatch {
    pub title_changed: bool,
    pub title: Option<String>,
    pub cursor_changed: bool,
    pub cursor: Option<NodeScreenCursor>,
    pub line_updates: Vec<NodeScreenLinePatch>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeScreenDelta {
    pub pane_id: String,
    pub from_sequence: u64,
    pub to_sequence: u64,
    pub rows: u16,
    pub cols: u16,
    pub source: NodeProjectionSource,
    pub patch: Option<NodeScreenPatch>,
    pub full_replace: Option<NodeScreenSurface>,
}
