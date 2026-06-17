use super::prelude::*;
use std::collections::BTreeMap;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum NodeScreenBufferKind {
    Normal,
    Alternate,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeScreenCursor {
    pub row: u16,
    pub col: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub shape: Option<NodeScreenCursorShape>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub blinking: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum NodeScreenCursorShape {
    Block,
    Underline,
    Beam,
    HollowBlock,
    Hidden,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export)]
pub enum NodeScreenColor {
    Named { name: String },
    Indexed { index: u8 },
    Rgb { r: u8, g: u8, b: u8 },
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeScreenSurfacePalette {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub foreground: Option<NodeScreenColor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub background: Option<NodeScreenColor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub cursor: Option<NodeScreenColor>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeScreenProgress {
    pub state: NodeScreenProgressState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub value: Option<u8>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum NodeScreenProgressState {
    #[default]
    Inactive,
    Normal,
    Error,
    Indeterminate,
    Warning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum NodeScreenUnderlineStyle {
    Single,
    Double,
    Curly,
    Dotted,
    Dashed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum NodeScreenTextBorderStyle {
    Framed,
    Encircled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum NodeScreenTextBaseline {
    Superscript,
    Subscript,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeScreenTextStyle {
    pub foreground: Option<NodeScreenColor>,
    pub background: Option<NodeScreenColor>,
    pub underline_color: Option<NodeScreenColor>,
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub blink: bool,
    pub underline: Option<NodeScreenUnderlineStyle>,
    pub overline: bool,
    pub border: Option<NodeScreenTextBorderStyle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub baseline: Option<NodeScreenTextBaseline>,
    pub inverse: bool,
    pub hidden: bool,
    pub strikethrough: bool,
    pub hyperlink: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeScreenLineSpan {
    pub text: String,
    pub style: NodeScreenTextStyle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum NodeScreenLineMediaKind {
    KittyGraphics,
    Iterm2Image,
    Sixel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeScreenLineMedia {
    pub kind: NodeScreenLineMediaKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub byte_size: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub width: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub height: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub preserve_aspect_ratio: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub inline: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub mime_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub data_base64: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub truncated: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum NodeScreenLineSideEffectKind {
    ClipboardWrite,
    ClipboardRead,
    DesktopNotification,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum NodeScreenLineSideEffectDisposition {
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum NodeScreenLineSideEffectTarget {
    Clipboard,
    Selection,
    DesktopNotification,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeScreenLineSideEffect {
    pub kind: NodeScreenLineSideEffectKind,
    pub disposition: NodeScreenLineSideEffectDisposition,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub target: Option<NodeScreenLineSideEffectTarget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum NodeScreenLineSemanticMarkKind {
    PromptStart,
    InputStart,
    OutputStart,
    CommandFinished,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeScreenLineSemanticMark {
    pub kind: NodeScreenLineSemanticMarkKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub col: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub exit_code: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeScreenLine {
    pub text: String,
    pub spans: Vec<NodeScreenLineSpan>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub media: Option<Vec<NodeScreenLineMedia>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub side_effects: Option<Vec<NodeScreenLineSideEffect>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub semantic_marks: Option<Vec<NodeScreenLineSemanticMark>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub wrapped: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NodeScreenSurface {
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub working_directory_uri: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub user_variables: Option<BTreeMap<String, String>>,
    pub cursor: Option<NodeScreenCursor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub palette: Option<NodeScreenSurfacePalette>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub bell_count: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub progress: Option<NodeScreenProgress>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub buffer_kind: Option<NodeScreenBufferKind>,
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
    pub working_directory_uri_changed: bool,
    pub working_directory_uri: Option<String>,
    pub user_variables_changed: bool,
    pub user_variables: Option<BTreeMap<String, String>>,
    pub cursor_changed: bool,
    pub cursor: Option<NodeScreenCursor>,
    pub palette_changed: bool,
    pub palette: Option<NodeScreenSurfacePalette>,
    pub bell_count_changed: bool,
    pub bell_count: Option<u64>,
    pub progress_changed: bool,
    pub progress: Option<NodeScreenProgress>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub buffer_kind: Option<NodeScreenBufferKind>,
    pub patch: Option<NodeScreenPatch>,
    pub full_replace: Option<NodeScreenSurface>,
}
