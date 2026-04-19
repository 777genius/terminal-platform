use serde::{Deserialize, Serialize};

use terminal_domain::PaneId;

use crate::{ProjectionSource, ScreenCursor, ScreenLine, ScreenSnapshot, ScreenSurface};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenLinePatch {
    pub row: u16,
    pub line: ScreenLine,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenPatch {
    pub title_changed: bool,
    pub title: Option<String>,
    pub cursor_changed: bool,
    pub cursor: Option<ScreenCursor>,
    pub line_updates: Vec<ScreenLinePatch>,
}

impl ScreenPatch {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        !self.title_changed && !self.cursor_changed && self.line_updates.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenDelta {
    pub pane_id: PaneId,
    pub from_sequence: u64,
    pub to_sequence: u64,
    pub rows: u16,
    pub cols: u16,
    pub source: ProjectionSource,
    pub patch: Option<ScreenPatch>,
    pub full_replace: Option<ScreenSurface>,
}

impl ScreenDelta {
    #[must_use]
    pub fn unchanged_from(snapshot: &ScreenSnapshot) -> Self {
        Self {
            pane_id: snapshot.pane_id,
            from_sequence: snapshot.sequence,
            to_sequence: snapshot.sequence,
            rows: snapshot.rows,
            cols: snapshot.cols,
            source: snapshot.source,
            patch: None,
            full_replace: None,
        }
    }

    #[must_use]
    pub fn full_replace(from_sequence: u64, snapshot: &ScreenSnapshot) -> Self {
        Self {
            pane_id: snapshot.pane_id,
            from_sequence,
            to_sequence: snapshot.sequence,
            rows: snapshot.rows,
            cols: snapshot.cols,
            source: snapshot.source,
            patch: None,
            full_replace: Some(snapshot.surface.clone()),
        }
    }

    #[must_use]
    pub fn between(previous: &ScreenSnapshot, current: &ScreenSnapshot) -> Self {
        if previous.pane_id != current.pane_id
            || previous.rows != current.rows
            || previous.cols != current.cols
            || previous.source != current.source
        {
            return Self::full_replace(previous.sequence, current);
        }

        let mut line_updates = Vec::new();
        let total_rows = previous.surface.lines.len().max(current.surface.lines.len());

        for row in 0..total_rows {
            let previous_line = previous.surface.lines.get(row);
            let current_line = current.surface.lines.get(row);
            if previous_line != current_line {
                line_updates.push(ScreenLinePatch {
                    row: row as u16,
                    line: current_line
                        .cloned()
                        .unwrap_or_else(|| ScreenLine { text: String::new() }),
                });
            }
        }

        let patch = ScreenPatch {
            title_changed: previous.surface.title != current.surface.title,
            title: current.surface.title.clone(),
            cursor_changed: previous.surface.cursor != current.surface.cursor,
            cursor: current.surface.cursor.clone(),
            line_updates,
        };

        Self {
            pane_id: current.pane_id,
            from_sequence: previous.sequence,
            to_sequence: current.sequence,
            rows: current.rows,
            cols: current.cols,
            source: current.source,
            patch: (!patch.is_empty()).then_some(patch),
            full_replace: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use terminal_domain::PaneId;

    use crate::{ProjectionSource, ScreenCursor, ScreenLine, ScreenSnapshot, ScreenSurface};

    use super::ScreenDelta;

    fn snapshot(
        pane_id: PaneId,
        sequence: u64,
        title: Option<&str>,
        lines: &[&str],
    ) -> ScreenSnapshot {
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
}
