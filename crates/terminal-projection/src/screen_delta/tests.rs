use terminal_domain::PaneId;

use crate::{ProjectionSource, ScreenCursor, ScreenLine, ScreenSnapshot, ScreenSurface};

use super::ScreenDelta;

fn snapshot(pane_id: PaneId, sequence: u64, title: Option<&str>, lines: &[&str]) -> ScreenSnapshot {
    ScreenSnapshot {
        pane_id,
        sequence,
        rows: 24,
        cols: 80,
        source: ProjectionSource::NativeEmulator,
        surface: ScreenSurface {
            title: title.map(str::to_string),
            cursor: Some(ScreenCursor { row: 0, col: 0 }),
            lines: lines.iter().map(|line| ScreenLine { text: (*line).to_string() }).collect(),
        },
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
