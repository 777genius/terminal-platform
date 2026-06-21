use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use terminal_domain::PaneId;

use crate::ProjectionSource;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScreenBufferKind {
    #[default]
    Normal,
    Alternate,
    Unknown,
}

impl ScreenBufferKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Alternate => "alternate",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ScreenCursor {
    pub row: u16,
    pub col: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<ScreenCursorShape>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub blinking: bool,
}

impl ScreenCursor {
    pub fn at(row: u16, col: u16) -> Self {
        Self { row, col, shape: None, blinking: false }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScreenCursorShape {
    Block,
    Underline,
    Beam,
    HollowBlock,
    Hidden,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ScreenLine {
    pub text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub spans: Vec<ScreenLineSpan>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub media: Vec<ScreenLineMedia>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub side_effects: Vec<ScreenLineSideEffect>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub semantic_marks: Vec<ScreenLineSemanticMark>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub wrapped: bool,
}

impl ScreenLine {
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            spans: Vec::new(),
            media: Vec::new(),
            side_effects: Vec::new(),
            semantic_marks: Vec::new(),
            wrapped: false,
        }
    }

    pub fn has_rich_content(&self) -> bool {
        self.wrapped
            || !self.media.is_empty()
            || !self.side_effects.is_empty()
            || !self.semantic_marks.is_empty()
            || self.spans.iter().any(|span| !span.style.is_plain())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ScreenLineMedia {
    pub kind: ScreenLineMediaKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub byte_size: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preserve_aspect_ratio: Option<bool>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub inline: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_base64: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub truncated: bool,
}

impl ScreenLineMedia {
    pub fn marker(kind: ScreenLineMediaKind) -> Self {
        Self {
            kind,
            name: None,
            byte_size: None,
            width: None,
            height: None,
            preserve_aspect_ratio: None,
            inline: false,
            mime_type: None,
            data_base64: None,
            truncated: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScreenLineMediaKind {
    KittyGraphics,
    Iterm2Image,
    Sixel,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ScreenLineSideEffect {
    pub kind: ScreenLineSideEffectKind,
    pub disposition: ScreenLineSideEffectDisposition,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<ScreenLineSideEffectTarget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScreenLineSideEffectKind {
    ClipboardWrite,
    ClipboardRead,
    DesktopNotification,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScreenLineSideEffectDisposition {
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScreenLineSideEffectTarget {
    Clipboard,
    Selection,
    DesktopNotification,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ScreenLineSemanticMark {
    pub kind: ScreenLineSemanticMarkKind,
    #[serde(default, skip_serializing_if = "is_zero_u16")]
    pub col: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScreenLineSemanticMarkKind {
    PromptStart,
    InputStart,
    OutputStart,
    CommandFinished,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ScreenLineSpan {
    pub text: String,
    #[serde(default, skip_serializing_if = "ScreenTextStyle::is_plain")]
    pub style: ScreenTextStyle,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ScreenTextStyle {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub foreground: Option<ScreenColor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<ScreenColor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub underline_color: Option<ScreenColor>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub bold: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub dim: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub italic: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub blink: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub underline: Option<ScreenUnderlineStyle>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub overline: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border: Option<ScreenTextBorderStyle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub baseline: Option<ScreenTextBaseline>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub inverse: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub hidden: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub strikethrough: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hyperlink: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScreenTextBaseline {
    Superscript,
    Subscript,
}

impl ScreenTextStyle {
    pub fn is_plain(&self) -> bool {
        self == &Self::default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ScreenColor {
    Named { name: String },
    Indexed { index: u8 },
    Rgb { r: u8, g: u8, b: u8 },
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ScreenSurfacePalette {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub foreground: Option<ScreenColor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<ScreenColor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<ScreenColor>,
}

impl ScreenSurfacePalette {
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ScreenProgress {
    pub state: ScreenProgressState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<u8>,
}

impl ScreenProgress {
    pub fn is_inactive(&self) -> bool {
        self == &Self::default()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScreenProgressState {
    #[default]
    Inactive,
    Normal,
    Error,
    Indeterminate,
    Warning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScreenUnderlineStyle {
    Single,
    Double,
    Curly,
    Dotted,
    Dashed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScreenTextBorderStyle {
    Framed,
    Encircled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenSurface {
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub working_directory_uri: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub user_variables: BTreeMap<String, String>,
    pub cursor: Option<ScreenCursor>,
    #[serde(default, skip_serializing_if = "ScreenSurfacePalette::is_empty")]
    pub palette: ScreenSurfacePalette,
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub bell_count: u64,
    #[serde(default, skip_serializing_if = "ScreenProgress::is_inactive")]
    pub progress: ScreenProgress,
    pub lines: Vec<ScreenLine>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenSnapshot {
    pub pane_id: PaneId,
    pub sequence: u64,
    pub rows: u16,
    pub cols: u16,
    pub source: ProjectionSource,
    #[serde(default, skip_serializing_if = "is_normal_buffer_kind")]
    pub buffer_kind: ScreenBufferKind,
    pub surface: ScreenSurface,
}

fn is_false(value: &bool) -> bool {
    !*value
}

fn is_zero_u64(value: &u64) -> bool {
    *value == 0
}

fn is_zero_u16(value: &u16) -> bool {
    *value == 0
}

fn is_normal_buffer_kind(value: &ScreenBufferKind) -> bool {
    *value == ScreenBufferKind::Normal
}

#[cfg(test)]
mod tests {
    use serde::{Serialize, de::DeserializeOwned};

    use super::*;

    fn parse_json<T: DeserializeOwned>(value: &str) -> T {
        serde_json::from_str(value).expect("test json should deserialize")
    }

    fn to_json<T: Serialize>(value: T) -> serde_json::Value {
        serde_json::to_value(value).expect("test value should serialize")
    }

    #[test]
    fn screen_line_defaults_missing_spans_for_legacy_payloads() {
        let line: ScreenLine = parse_json(r#"{"text":"ready"}"#);

        assert_eq!(line, ScreenLine::plain("ready"));
    }

    #[test]
    fn screen_cursor_defaults_missing_shape_for_legacy_payloads() {
        let cursor: ScreenCursor = parse_json(r#"{"row":2,"col":8}"#);

        assert_eq!(cursor, ScreenCursor::at(2, 8));
    }

    #[test]
    fn screen_snapshot_defaults_missing_buffer_kind_for_legacy_payloads() {
        let snapshot: ScreenSnapshot = parse_json(
            r#"{"pane_id":"00000000-0000-0000-0000-000000000001","sequence":1,"rows":24,"cols":80,"source":"native_emulator","surface":{"title":null,"cursor":null,"lines":[]}}"#,
        );

        assert_eq!(snapshot.buffer_kind, ScreenBufferKind::Normal);
        assert_eq!(snapshot.surface.palette, ScreenSurfacePalette::default());
    }

    #[test]
    fn screen_snapshot_serializes_non_default_buffer_kind_only() {
        let snapshot = ScreenSnapshot {
            pane_id: PaneId::new(),
            sequence: 1,
            rows: 24,
            cols: 80,
            source: ProjectionSource::NativeEmulator,
            buffer_kind: ScreenBufferKind::Alternate,
            surface: ScreenSurface {
                title: None,
                working_directory_uri: None,
                user_variables: Default::default(),
                cursor: None,
                palette: ScreenSurfacePalette::default(),
                bell_count: 0,
                progress: Default::default(),
                lines: Vec::new(),
            },
        };

        let value = to_json(snapshot);

        assert_eq!(value["buffer_kind"], "alternate");
    }

    #[test]
    fn screen_cursor_omits_default_shape_fields() {
        let value = to_json(ScreenCursor::at(2, 8));

        assert_eq!(value, serde_json::json!({ "row": 2, "col": 8 }));
    }

    #[test]
    fn screen_cursor_serializes_shape_and_blinking_when_present() {
        let value = to_json(ScreenCursor {
            row: 2,
            col: 8,
            shape: Some(ScreenCursorShape::Beam),
            blinking: true,
        });

        assert_eq!(
            value,
            serde_json::json!({
                "row": 2,
                "col": 8,
                "shape": "beam",
                "blinking": true
            })
        );
    }

    #[test]
    fn screen_surface_omits_default_palette_and_serializes_overrides() {
        let default_value = to_json(ScreenSurface {
            title: None,
            working_directory_uri: None,
            user_variables: Default::default(),
            cursor: None,
            palette: ScreenSurfacePalette::default(),
            bell_count: 0,
            progress: Default::default(),
            lines: Vec::new(),
        });

        assert!(default_value.get("palette").is_none());
        assert!(default_value.get("working_directory_uri").is_none());

        let override_value = to_json(ScreenSurface {
            title: None,
            working_directory_uri: None,
            user_variables: Default::default(),
            cursor: None,
            palette: ScreenSurfacePalette {
                foreground: Some(ScreenColor::Rgb { r: 1, g: 2, b: 3 }),
                background: None,
                cursor: None,
            },
            bell_count: 0,
            progress: Default::default(),
            lines: Vec::new(),
        });

        assert_eq!(
            override_value["palette"]["foreground"],
            serde_json::json!({ "kind": "rgb", "r": 1, "g": 2, "b": 3 })
        );
    }

    #[test]
    fn screen_surface_serializes_working_directory_uri_only_when_present() {
        let value = to_json(ScreenSurface {
            title: None,
            working_directory_uri: Some("file://localhost/tmp/project".to_string()),
            user_variables: Default::default(),
            cursor: None,
            palette: ScreenSurfacePalette::default(),
            bell_count: 0,
            progress: Default::default(),
            lines: Vec::new(),
        });

        assert_eq!(value["working_directory_uri"], "file://localhost/tmp/project");
    }

    #[test]
    fn screen_surface_omits_default_bell_count_and_serializes_nonzero() {
        let default_value = to_json(ScreenSurface {
            title: None,
            working_directory_uri: None,
            user_variables: Default::default(),
            cursor: None,
            palette: ScreenSurfacePalette::default(),
            bell_count: 0,
            progress: Default::default(),
            lines: Vec::new(),
        });
        assert!(default_value.get("bell_count").is_none());

        let bell_value = to_json(ScreenSurface {
            title: None,
            working_directory_uri: None,
            user_variables: Default::default(),
            cursor: None,
            palette: ScreenSurfacePalette::default(),
            bell_count: 2,
            progress: Default::default(),
            lines: Vec::new(),
        });
        assert_eq!(bell_value["bell_count"], serde_json::json!(2));
    }

    #[test]
    fn screen_surface_omits_inactive_progress_and_serializes_active_progress() {
        let default_value = to_json(ScreenSurface {
            title: None,
            working_directory_uri: None,
            user_variables: Default::default(),
            cursor: None,
            palette: ScreenSurfacePalette::default(),
            bell_count: 0,
            progress: ScreenProgress::default(),
            lines: Vec::new(),
        });
        assert!(default_value.get("progress").is_none());

        let progress_value = to_json(ScreenSurface {
            title: None,
            working_directory_uri: None,
            user_variables: Default::default(),
            cursor: None,
            palette: ScreenSurfacePalette::default(),
            bell_count: 0,
            progress: ScreenProgress { state: ScreenProgressState::Warning, value: Some(88) },
            lines: Vec::new(),
        });
        assert_eq!(
            progress_value["progress"],
            serde_json::json!({ "state": "warning", "value": 88 })
        );
    }

    #[test]
    fn screen_surface_omits_empty_user_variables_and_serializes_nonempty() {
        let default_value = to_json(ScreenSurface {
            title: None,
            working_directory_uri: None,
            user_variables: Default::default(),
            cursor: None,
            palette: ScreenSurfacePalette::default(),
            bell_count: 0,
            progress: ScreenProgress::default(),
            lines: Vec::new(),
        });
        assert!(default_value.get("user_variables").is_none());

        let value = to_json(ScreenSurface {
            title: None,
            working_directory_uri: None,
            user_variables: [("WEZTERM_PROG".to_string(), "cargo test".to_string())].into(),
            cursor: None,
            palette: ScreenSurfacePalette::default(),
            bell_count: 0,
            progress: ScreenProgress::default(),
            lines: Vec::new(),
        });
        assert_eq!(value["user_variables"]["WEZTERM_PROG"], "cargo test");
    }

    #[test]
    fn screen_line_omits_empty_spans_for_plain_payloads() {
        let value = to_json(ScreenLine::plain("ready"));

        assert_eq!(value, serde_json::json!({ "text": "ready" }));
    }

    #[test]
    fn screen_line_defaults_missing_wrapped_for_legacy_payloads() {
        let line: ScreenLine = parse_json(r#"{"text":"ready","spans":[]}"#);

        assert!(!line.wrapped);
        assert!(line.media.is_empty());
        assert!(line.side_effects.is_empty());
        assert!(line.semantic_marks.is_empty());
    }

    #[test]
    fn screen_line_serializes_wrapped_only_when_true() {
        let value = to_json(ScreenLine {
            text: "wrapped".to_string(),
            spans: Vec::new(),
            media: Vec::new(),
            side_effects: Vec::new(),
            semantic_marks: Vec::new(),
            wrapped: true,
        });

        assert_eq!(value, serde_json::json!({ "text": "wrapped", "wrapped": true }));
    }

    #[test]
    fn screen_line_serializes_rich_spans_only_when_present() {
        let line = ScreenLine {
            text: "red link".to_string(),
            spans: vec![
                ScreenLineSpan {
                    text: "red".to_string(),
                    style: ScreenTextStyle {
                        foreground: Some(ScreenColor::Named { name: "red".to_string() }),
                        bold: true,
                        ..ScreenTextStyle::default()
                    },
                },
                ScreenLineSpan {
                    text: " link".to_string(),
                    style: ScreenTextStyle {
                        hyperlink: Some("https://example.com".to_string()),
                        underline: Some(ScreenUnderlineStyle::Single),
                        overline: true,
                        blink: true,
                        border: Some(ScreenTextBorderStyle::Framed),
                        ..ScreenTextStyle::default()
                    },
                },
            ],
            media: Vec::new(),
            side_effects: Vec::new(),
            semantic_marks: Vec::new(),
            wrapped: false,
        };

        let value = to_json(line);

        assert_eq!(
            value,
            serde_json::json!({
                "text": "red link",
                "spans": [
                    {
                        "text": "red",
                        "style": {
                            "foreground": { "kind": "named", "name": "red" },
                            "bold": true
                        }
                    },
                    {
                        "text": " link",
                        "style": {
                            "underline": "single",
                            "overline": true,
                            "blink": true,
                            "border": "framed",
                            "hyperlink": "https://example.com"
                        }
                    }
                ]
            })
        );
    }

    #[test]
    fn screen_line_serializes_media_markers_only_when_present() {
        let value = to_json(ScreenLine {
            text: String::new(),
            spans: Vec::new(),
            media: vec![ScreenLineMedia::marker(ScreenLineMediaKind::KittyGraphics)],
            side_effects: Vec::new(),
            semantic_marks: Vec::new(),
            wrapped: false,
        });

        assert_eq!(
            value,
            serde_json::json!({
                "text": "",
                "media": [{ "kind": "kitty_graphics" }]
            })
        );
    }

    #[test]
    fn screen_line_serializes_side_effect_markers_only_when_present() {
        let value = to_json(ScreenLine {
            text: String::new(),
            spans: Vec::new(),
            media: Vec::new(),
            side_effects: vec![ScreenLineSideEffect {
                kind: ScreenLineSideEffectKind::ClipboardWrite,
                disposition: ScreenLineSideEffectDisposition::Blocked,
                target: Some(ScreenLineSideEffectTarget::Clipboard),
                message: None,
            }],
            semantic_marks: Vec::new(),
            wrapped: false,
        });

        assert_eq!(
            value,
            serde_json::json!({
                "text": "",
                "side_effects": [{
                    "kind": "clipboard_write",
                    "disposition": "blocked",
                    "target": "clipboard"
                }]
            })
        );
    }

    #[test]
    fn screen_line_serializes_semantic_marks_only_when_present() {
        let value = to_json(ScreenLine {
            text: String::new(),
            spans: Vec::new(),
            media: Vec::new(),
            side_effects: Vec::new(),
            semantic_marks: vec![
                ScreenLineSemanticMark {
                    kind: ScreenLineSemanticMarkKind::PromptStart,
                    col: 0,
                    exit_code: None,
                },
                ScreenLineSemanticMark {
                    kind: ScreenLineSemanticMarkKind::CommandFinished,
                    col: 12,
                    exit_code: Some(127),
                },
            ],
            wrapped: false,
        });

        assert_eq!(
            value,
            serde_json::json!({
                "text": "",
                "semantic_marks": [
                    { "kind": "prompt_start" },
                    { "kind": "command_finished", "col": 12, "exit_code": 127 }
                ]
            })
        );
    }
}
