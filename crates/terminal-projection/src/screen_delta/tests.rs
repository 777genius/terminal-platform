use terminal_domain::PaneId;

use crate::{
    ProjectionSource, ScreenBufferKind, ScreenColor, ScreenCursor, ScreenLine,
    ScreenLineSemanticMark, ScreenLineSemanticMarkKind, ScreenLineSideEffect,
    ScreenLineSideEffectDisposition, ScreenLineSideEffectKind, ScreenLineSideEffectTarget,
    ScreenLineSpan, ScreenProgress, ScreenProgressState, ScreenSnapshot, ScreenSurface,
    ScreenSurfacePalette, ScreenTextStyle,
};

use super::ScreenDelta;

fn snapshot(pane_id: PaneId, sequence: u64, title: Option<&str>, lines: &[&str]) -> ScreenSnapshot {
    ScreenSnapshot {
        pane_id,
        sequence,
        rows: 24,
        cols: 80,
        source: ProjectionSource::NativeEmulator,
        buffer_kind: ScreenBufferKind::Normal,
        surface: ScreenSurface {
            title: title.map(str::to_string),
            working_directory_uri: None,
            user_variables: Default::default(),
            cursor: Some(ScreenCursor::at(0, 0)),
            palette: Default::default(),
            bell_count: 0,
            progress: Default::default(),
            lines: lines.iter().map(|line| ScreenLine::plain(*line)).collect(),
        },
    }
}

fn snapshot_with_screen_lines(
    pane_id: PaneId,
    sequence: u64,
    lines: Vec<ScreenLine>,
) -> ScreenSnapshot {
    ScreenSnapshot {
        pane_id,
        sequence,
        rows: 24,
        cols: 80,
        source: ProjectionSource::NativeEmulator,
        buffer_kind: ScreenBufferKind::Normal,
        surface: ScreenSurface {
            title: Some("shell".to_string()),
            working_directory_uri: None,
            user_variables: Default::default(),
            cursor: Some(ScreenCursor::at(0, 0)),
            palette: Default::default(),
            bell_count: 0,
            progress: Default::default(),
            lines,
        },
    }
}

fn rich_line(text: &str, style: ScreenTextStyle) -> ScreenLine {
    ScreenLine {
        text: text.to_string(),
        spans: vec![ScreenLineSpan { text: text.to_string(), style }],
        media: Vec::new(),
        side_effects: Vec::new(),
        semantic_marks: Vec::new(),
        wrapped: false,
    }
}

#[test]
fn computes_line_and_title_patch_between_snapshots() {
    let pane_id = PaneId::new();
    let previous = snapshot(pane_id, 4, Some("shell"), &["ready", ""]);
    let current = snapshot(pane_id, 5, Some("logs"), &["ready", "hello"]);
    let delta = ScreenDelta::between(&previous, &current);

    assert_eq!(delta.from_sequence, 4);
    assert_eq!(delta.to_sequence, 5);
    assert!(delta.full_replace.is_none());
    let patch = delta.patch.expect("patch should exist");
    assert!(patch.title_changed);
    assert_eq!(patch.title.as_deref(), Some("logs"));
    assert_eq!(patch.line_updates.len(), 1);
    assert_eq!(patch.line_updates[0].row, 1);
    assert_eq!(patch.line_updates[0].line.text, "hello");
}

#[test]
fn returns_empty_patch_for_identical_snapshots() {
    let pane_id = PaneId::new();
    let previous = snapshot(pane_id, 7, Some("shell"), &["ready"]);
    let current = ScreenSnapshot { sequence: 8, ..previous.clone() };
    let delta = ScreenDelta::between(&previous, &current);

    assert!(delta.patch.is_none());
    assert!(delta.full_replace.is_none());
}

#[test]
fn computes_working_directory_uri_patch_between_snapshots() {
    let pane_id = PaneId::new();
    let previous = snapshot(pane_id, 8, Some("shell"), &["ready"]);
    let mut current = ScreenSnapshot { sequence: 9, ..previous.clone() };
    current.surface.working_directory_uri = Some("file://localhost/tmp/project".to_string());

    let delta = ScreenDelta::between(&previous, &current);

    assert!(delta.full_replace.is_none());
    let patch = delta.patch.expect("working directory metadata should patch");
    assert!(patch.working_directory_uri_changed);
    assert_eq!(patch.working_directory_uri.as_deref(), Some("file://localhost/tmp/project"));
    assert!(!patch.title_changed);
    assert!(patch.line_updates.is_empty());
}

#[test]
fn computes_user_variables_patch_between_snapshots() {
    let pane_id = PaneId::new();
    let previous = snapshot(pane_id, 8, Some("shell"), &["ready"]);
    let mut current = ScreenSnapshot { sequence: 9, ..previous.clone() };
    current.surface.user_variables.insert("WEZTERM_PROG".to_string(), "cargo test".to_string());

    let delta = ScreenDelta::between(&previous, &current);

    assert!(delta.full_replace.is_none());
    let patch = delta.patch.expect("user variable metadata should patch");
    assert!(patch.user_variables_changed);
    assert_eq!(
        patch.user_variables.as_ref().and_then(|vars| vars.get("WEZTERM_PROG")),
        Some(&"cargo test".to_string())
    );
    assert!(!patch.working_directory_uri_changed);
    assert!(patch.line_updates.is_empty());
}

#[test]
fn computes_progress_patch_between_snapshots() {
    let pane_id = PaneId::new();
    let previous = snapshot(pane_id, 8, Some("shell"), &["ready"]);
    let mut current = ScreenSnapshot { sequence: 9, ..previous.clone() };
    current.surface.progress =
        ScreenProgress { state: ScreenProgressState::Normal, value: Some(42) };

    let delta = ScreenDelta::between(&previous, &current);

    assert!(delta.full_replace.is_none());
    let patch = delta.patch.expect("progress metadata should patch");
    assert!(patch.progress_changed);
    assert_eq!(
        patch.progress,
        Some(ScreenProgress { state: ScreenProgressState::Normal, value: Some(42) })
    );
    assert!(patch.line_updates.is_empty());

    let mut cleared = ScreenSnapshot { sequence: 10, ..current.clone() };
    cleared.surface.progress = ScreenProgress::default();
    let clear_delta = ScreenDelta::between(&current, &cleared);

    let clear_patch = clear_delta.patch.expect("progress clear should patch");
    assert!(clear_patch.progress_changed);
    assert_eq!(clear_patch.progress, None);
}

#[test]
fn computes_line_patch_when_only_rich_style_changes() {
    let pane_id = PaneId::new();
    let previous = snapshot_with_screen_lines(pane_id, 8, vec![ScreenLine::plain("ready")]);
    let current = snapshot_with_screen_lines(
        pane_id,
        9,
        vec![rich_line(
            "ready",
            ScreenTextStyle {
                foreground: Some(ScreenColor::Named { name: "green".to_string() }),
                bold: true,
                ..ScreenTextStyle::default()
            },
        )],
    );
    let delta = ScreenDelta::between(&previous, &current);

    assert!(delta.full_replace.is_none());
    let patch = delta.patch.expect("style-only change should update the row");
    assert_eq!(patch.line_updates.len(), 1);
    assert_eq!(patch.line_updates[0].row, 0);
    assert_eq!(patch.line_updates[0].line, current.surface.lines[0]);
}

#[test]
fn computes_line_patch_when_only_soft_wrap_metadata_changes() {
    let pane_id = PaneId::new();
    let previous = snapshot_with_screen_lines(pane_id, 8, vec![ScreenLine::plain("wrapped")]);
    let current = snapshot_with_screen_lines(
        pane_id,
        9,
        vec![ScreenLine {
            text: "wrapped".to_string(),
            spans: Vec::new(),
            media: Vec::new(),
            side_effects: Vec::new(),
            semantic_marks: Vec::new(),
            wrapped: true,
        }],
    );
    let delta = ScreenDelta::between(&previous, &current);

    assert!(delta.full_replace.is_none());
    let patch = delta.patch.expect("wrapped-only change should update the row");
    assert_eq!(patch.line_updates.len(), 1);
    assert_eq!(patch.line_updates[0].row, 0);
    assert!(patch.line_updates[0].line.wrapped);
}

#[test]
fn computes_line_patch_when_only_side_effect_metadata_changes() {
    let pane_id = PaneId::new();
    let previous = snapshot_with_screen_lines(pane_id, 10, vec![ScreenLine::plain("")]);
    let current = snapshot_with_screen_lines(
        pane_id,
        11,
        vec![ScreenLine {
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
        }],
    );
    let delta = ScreenDelta::between(&previous, &current);

    assert!(delta.full_replace.is_none());
    let patch = delta.patch.expect("side-effect-only change should update the row");
    assert_eq!(patch.line_updates.len(), 1);
    assert_eq!(patch.line_updates[0].row, 0);
    assert_eq!(patch.line_updates[0].line, current.surface.lines[0]);
}

#[test]
fn computes_line_patch_when_only_semantic_metadata_changes() {
    let pane_id = PaneId::new();
    let previous = snapshot_with_screen_lines(pane_id, 12, vec![ScreenLine::plain("")]);
    let current = snapshot_with_screen_lines(
        pane_id,
        13,
        vec![ScreenLine {
            text: String::new(),
            spans: Vec::new(),
            media: Vec::new(),
            side_effects: Vec::new(),
            semantic_marks: vec![ScreenLineSemanticMark {
                kind: ScreenLineSemanticMarkKind::CommandFinished,
                col: 0,
                exit_code: Some(1),
            }],
            wrapped: false,
        }],
    );
    let delta = ScreenDelta::between(&previous, &current);

    assert!(delta.full_replace.is_none());
    let patch = delta.patch.expect("semantic-only change should update the row");
    assert_eq!(patch.line_updates.len(), 1);
    assert_eq!(patch.line_updates[0].row, 0);
    assert_eq!(patch.line_updates[0].line, current.surface.lines[0]);
}

#[test]
fn falls_back_to_full_replace_for_buffer_kind_change() {
    let pane_id = PaneId::new();
    let previous = snapshot(pane_id, 2, Some("shell"), &["ready"]);
    let mut current = snapshot(pane_id, 3, Some("shell"), &["ready"]);
    current.buffer_kind = ScreenBufferKind::Alternate;

    let delta = ScreenDelta::between(&previous, &current);

    assert!(delta.patch.is_none());
    assert!(delta.full_replace.is_some());
    assert_eq!(delta.buffer_kind, ScreenBufferKind::Alternate);
}

#[test]
fn computes_surface_palette_patch_between_snapshots() {
    let pane_id = PaneId::new();
    let previous = snapshot(pane_id, 2, Some("shell"), &["ready"]);
    let mut current = snapshot(pane_id, 3, Some("shell"), &["ready"]);
    current.surface.palette = ScreenSurfacePalette {
        foreground: Some(ScreenColor::Rgb { r: 1, g: 2, b: 3 }),
        background: None,
        cursor: None,
    };

    let delta = ScreenDelta::between(&previous, &current);

    assert!(delta.full_replace.is_none());
    let patch = delta.patch.expect("surface palette metadata should patch");
    assert!(patch.palette_changed);
    assert_eq!(patch.palette, Some(current.surface.palette.clone()));
    assert!(patch.line_updates.is_empty());

    let mut cleared = ScreenSnapshot { sequence: 4, ..current.clone() };
    cleared.surface.palette = ScreenSurfacePalette::default();
    let clear_delta = ScreenDelta::between(&current, &cleared);
    let clear_patch = clear_delta.patch.expect("surface palette clear should patch");
    assert!(clear_patch.palette_changed);
    assert_eq!(clear_patch.palette, None);
}

#[test]
fn computes_bell_count_patch_between_snapshots() {
    let pane_id = PaneId::new();
    let previous = snapshot(pane_id, 2, Some("shell"), &["ready"]);
    let mut current = snapshot(pane_id, 3, Some("shell"), &["ready"]);
    current.surface.bell_count = 1;

    let delta = ScreenDelta::between(&previous, &current);

    assert!(delta.full_replace.is_none());
    let patch = delta.patch.expect("bell count metadata should patch");
    assert!(patch.bell_count_changed);
    assert_eq!(patch.bell_count, Some(1));
    assert!(patch.line_updates.is_empty());

    let mut cleared = ScreenSnapshot { sequence: 4, ..current.clone() };
    cleared.surface.bell_count = 0;
    let clear_delta = ScreenDelta::between(&current, &cleared);
    let clear_patch = clear_delta.patch.expect("bell count clear should patch");
    assert!(clear_patch.bell_count_changed);
    assert_eq!(clear_patch.bell_count, None);
}

#[test]
fn falls_back_to_full_replace_for_dimension_change() {
    let pane_id = PaneId::new();
    let previous = snapshot(pane_id, 2, Some("shell"), &["ready"]);
    let mut current = snapshot(pane_id, 3, Some("shell"), &["ready", "hello"]);
    current.rows = 40;

    let delta = ScreenDelta::between(&previous, &current);

    assert!(delta.patch.is_none());
    assert!(delta.full_replace.is_some());
    assert_eq!(delta.rows, 40);
}
