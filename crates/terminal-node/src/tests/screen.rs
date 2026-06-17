use terminal_projection::{
    ScreenColor, ScreenLine, ScreenLineMedia, ScreenLineMediaKind, ScreenLineSemanticMark,
    ScreenLineSemanticMarkKind, ScreenLineSideEffect, ScreenLineSideEffectDisposition,
    ScreenLineSideEffectKind, ScreenLineSideEffectTarget, ScreenLineSpan, ScreenPatch,
    ScreenProgress, ScreenProgressState, ScreenSurface, ScreenSurfacePalette, ScreenTextBaseline,
    ScreenTextBorderStyle, ScreenTextStyle, ScreenUnderlineStyle,
};

use crate::{
    NodeScreenColor, NodeScreenLine, NodeScreenLineMediaKind, NodeScreenLineSemanticMarkKind,
    NodeScreenLineSideEffectDisposition, NodeScreenLineSideEffectKind,
    NodeScreenLineSideEffectTarget, NodeScreenPatch, NodeScreenProgressState, NodeScreenSurface,
    NodeScreenTextBaseline, NodeScreenTextBorderStyle, NodeScreenUnderlineStyle,
};

#[test]
fn node_screen_surface_omits_zero_bell_count_and_preserves_nonzero() {
    let default_surface = ScreenSurface {
        title: None,
        working_directory_uri: None,
        user_variables: Default::default(),
        cursor: None,
        palette: Default::default(),
        bell_count: 0,
        progress: Default::default(),
        lines: vec![ScreenLine::plain("ready")],
    };
    let bell_surface = ScreenSurface { bell_count: 3, ..default_surface.clone() };

    assert_eq!(NodeScreenSurface::from(&default_surface).bell_count, None);
    assert_eq!(NodeScreenSurface::from(&bell_surface).bell_count, Some(3));
}

#[test]
fn node_screen_patch_preserves_palette_and_bell_metadata() {
    let patch = ScreenPatch {
        title_changed: false,
        title: None,
        working_directory_uri_changed: false,
        working_directory_uri: None,
        user_variables_changed: false,
        user_variables: None,
        cursor_changed: false,
        cursor: None,
        palette_changed: true,
        palette: Some(ScreenSurfacePalette {
            foreground: Some(ScreenColor::Rgb { r: 1, g: 2, b: 3 }),
            background: Some(ScreenColor::Rgb { r: 4, g: 5, b: 6 }),
            cursor: Some(ScreenColor::Rgb { r: 7, g: 8, b: 9 }),
        }),
        bell_count_changed: true,
        bell_count: Some(4),
        progress_changed: false,
        progress: None,
        line_updates: Vec::new(),
    };

    let node_patch = NodeScreenPatch::from(&patch);

    assert!(node_patch.palette_changed);
    let palette = node_patch.palette.expect("palette patch should survive");
    assert_eq!(palette.foreground, Some(NodeScreenColor::Rgb { r: 1, g: 2, b: 3 }));
    assert_eq!(palette.background, Some(NodeScreenColor::Rgb { r: 4, g: 5, b: 6 }));
    assert_eq!(palette.cursor, Some(NodeScreenColor::Rgb { r: 7, g: 8, b: 9 }));
    assert!(node_patch.bell_count_changed);
    assert_eq!(node_patch.bell_count, Some(4));
}

#[test]
fn node_screen_surface_omits_inactive_progress_and_preserves_active_progress() {
    let default_surface = ScreenSurface {
        title: None,
        working_directory_uri: None,
        user_variables: Default::default(),
        cursor: None,
        palette: Default::default(),
        bell_count: 0,
        progress: Default::default(),
        lines: vec![ScreenLine::plain("ready")],
    };
    let progress_surface = ScreenSurface {
        progress: ScreenProgress { state: ScreenProgressState::Warning, value: Some(88) },
        ..default_surface.clone()
    };

    assert_eq!(NodeScreenSurface::from(&default_surface).progress, None);
    let progress =
        NodeScreenSurface::from(&progress_surface).progress.expect("progress should survive");
    assert_eq!(progress.state, NodeScreenProgressState::Warning);
    assert_eq!(progress.value, Some(88));
}

#[test]
fn node_screen_surface_omits_empty_user_variables_and_preserves_nonempty() {
    let mut surface = ScreenSurface {
        title: None,
        working_directory_uri: None,
        user_variables: Default::default(),
        cursor: None,
        palette: Default::default(),
        bell_count: 0,
        progress: Default::default(),
        lines: vec![ScreenLine::plain("ready")],
    };

    assert_eq!(NodeScreenSurface::from(&surface).user_variables, None);

    surface.user_variables.insert("WEZTERM_PROG".to_string(), "cargo test".to_string());
    let vars =
        NodeScreenSurface::from(&surface).user_variables.expect("user variables should survive");
    assert_eq!(vars.get("WEZTERM_PROG"), Some(&"cargo test".to_string()));
}

#[test]
fn node_screen_line_preserves_rich_spans_and_text_styles() {
    let rich_line = ScreenLine {
        text: "warn link".to_string(),
        spans: vec![
            ScreenLineSpan {
                text: "warn".to_string(),
                style: ScreenTextStyle {
                    foreground: Some(ScreenColor::Named { name: "yellow".to_string() }),
                    background: Some(ScreenColor::Indexed { index: 52 }),
                    underline_color: Some(ScreenColor::Rgb { r: 1, g: 2, b: 3 }),
                    bold: true,
                    dim: true,
                    italic: true,
                    blink: true,
                    underline: Some(ScreenUnderlineStyle::Curly),
                    overline: true,
                    border: Some(ScreenTextBorderStyle::Framed),
                    baseline: Some(ScreenTextBaseline::Superscript),
                    inverse: true,
                    hidden: false,
                    strikethrough: true,
                    hyperlink: None,
                },
            },
            ScreenLineSpan {
                text: " link".to_string(),
                style: ScreenTextStyle {
                    foreground: Some(ScreenColor::Rgb { r: 10, g: 20, b: 30 }),
                    hyperlink: Some("https://example.test".to_string()),
                    ..ScreenTextStyle::default()
                },
            },
        ],
        media: Vec::new(),
        side_effects: Vec::new(),
        semantic_marks: Vec::new(),
        wrapped: false,
    };

    let node_line = NodeScreenLine::from(&rich_line);

    assert_eq!(node_line.spans.len(), 2);
    let warn = &node_line.spans[0];
    assert_eq!(warn.text, "warn");
    assert_eq!(warn.style.foreground, Some(NodeScreenColor::Named { name: "yellow".to_string() }));
    assert_eq!(warn.style.background, Some(NodeScreenColor::Indexed { index: 52 }));
    assert_eq!(warn.style.underline_color, Some(NodeScreenColor::Rgb { r: 1, g: 2, b: 3 }));
    assert!(warn.style.bold);
    assert!(warn.style.dim);
    assert!(warn.style.italic);
    assert!(warn.style.blink);
    assert_eq!(warn.style.underline, Some(NodeScreenUnderlineStyle::Curly));
    assert!(warn.style.overline);
    assert_eq!(warn.style.border, Some(NodeScreenTextBorderStyle::Framed));
    assert_eq!(warn.style.baseline, Some(NodeScreenTextBaseline::Superscript));
    assert!(warn.style.inverse);
    assert!(warn.style.strikethrough);

    let link = &node_line.spans[1];
    assert_eq!(link.text, " link");
    assert_eq!(link.style.foreground, Some(NodeScreenColor::Rgb { r: 10, g: 20, b: 30 }));
    assert_eq!(link.style.hyperlink.as_deref(), Some("https://example.test"));
}

#[test]
fn node_screen_line_omits_empty_media_and_preserves_media_markers() {
    let plain_line = ScreenLine::plain("ready");
    let media_line = ScreenLine {
        text: String::new(),
        spans: Vec::new(),
        media: vec![ScreenLineMedia {
            kind: ScreenLineMediaKind::Iterm2Image,
            name: Some("tiny.png".to_string()),
            byte_size: Some(68),
            width: Some("2".to_string()),
            height: Some("1".to_string()),
            preserve_aspect_ratio: Some(true),
            inline: true,
            mime_type: Some("image/png".to_string()),
            data_base64: Some("iVBORw0KGgo=".to_string()),
            truncated: false,
        }],
        side_effects: Vec::new(),
        semantic_marks: Vec::new(),
        wrapped: false,
    };

    assert_eq!(NodeScreenLine::from(&plain_line).media, None);
    let mut markers = NodeScreenLine::from(&media_line).media.expect("media marker should survive");
    let marker = markers.remove(0);
    assert_eq!(marker.kind, NodeScreenLineMediaKind::Iterm2Image);
    assert_eq!(marker.name.as_deref(), Some("tiny.png"));
    assert_eq!(marker.byte_size, Some(68));
    assert_eq!(marker.width.as_deref(), Some("2"));
    assert_eq!(marker.height.as_deref(), Some("1"));
    assert_eq!(marker.preserve_aspect_ratio, Some(true));
    assert_eq!(marker.inline, Some(true));
    assert_eq!(marker.mime_type.as_deref(), Some("image/png"));
    assert_eq!(marker.data_base64.as_deref(), Some("iVBORw0KGgo="));
    assert_eq!(marker.truncated, None);
}

#[test]
fn node_screen_line_omits_empty_side_effects_and_preserves_blocked_clipboard_markers() {
    let plain_line = ScreenLine::plain("ready");
    let side_effect_line = ScreenLine {
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
    };

    assert_eq!(NodeScreenLine::from(&plain_line).side_effects, None);
    let mut markers = NodeScreenLine::from(&side_effect_line)
        .side_effects
        .expect("side-effect marker should survive");
    let marker = markers.remove(0);
    assert_eq!(marker.kind, NodeScreenLineSideEffectKind::ClipboardWrite);
    assert_eq!(marker.disposition, NodeScreenLineSideEffectDisposition::Blocked);
    assert_eq!(marker.target, Some(NodeScreenLineSideEffectTarget::Clipboard));
}

#[test]
fn node_screen_line_omits_empty_semantic_marks_and_preserves_shell_integration_marks() {
    let plain_line = ScreenLine::plain("ready");
    let semantic_line = ScreenLine {
        text: String::new(),
        spans: Vec::new(),
        media: Vec::new(),
        side_effects: Vec::new(),
        semantic_marks: vec![ScreenLineSemanticMark {
            kind: ScreenLineSemanticMarkKind::CommandFinished,
            col: 3,
            exit_code: Some(127),
        }],
        wrapped: false,
    };

    assert_eq!(NodeScreenLine::from(&plain_line).semantic_marks, None);
    let mut markers =
        NodeScreenLine::from(&semantic_line).semantic_marks.expect("semantic mark should survive");
    let marker = markers.remove(0);
    assert_eq!(marker.kind, NodeScreenLineSemanticMarkKind::CommandFinished);
    assert_eq!(marker.col, Some(3));
    assert_eq!(marker.exit_code, Some(127));
}
