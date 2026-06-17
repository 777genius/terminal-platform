use crate::{ScreenLine, ScreenSnapshot};

use super::model::{ScreenDelta, ScreenLinePatch, ScreenPatch};

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
            buffer_kind: snapshot.buffer_kind,
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
            buffer_kind: snapshot.buffer_kind,
            patch: None,
            full_replace: Some(snapshot.surface.clone()),
        }
    }

    #[must_use]
    pub fn between(previous: &ScreenSnapshot, current: &ScreenSnapshot) -> Self {
        if requires_full_replace(previous, current) {
            return Self::full_replace(previous.sequence, current);
        }

        let patch = ScreenPatch {
            title_changed: previous.surface.title != current.surface.title,
            title: current.surface.title.clone(),
            working_directory_uri_changed: previous.surface.working_directory_uri
                != current.surface.working_directory_uri,
            working_directory_uri: current.surface.working_directory_uri.clone(),
            user_variables_changed: previous.surface.user_variables
                != current.surface.user_variables,
            user_variables: (!current.surface.user_variables.is_empty())
                .then_some(current.surface.user_variables.clone()),
            cursor_changed: previous.surface.cursor != current.surface.cursor,
            cursor: current.surface.cursor.clone(),
            palette_changed: previous.surface.palette != current.surface.palette,
            palette: (!current.surface.palette.is_empty())
                .then_some(current.surface.palette.clone()),
            bell_count_changed: previous.surface.bell_count != current.surface.bell_count,
            bell_count: (current.surface.bell_count > 0).then_some(current.surface.bell_count),
            progress_changed: previous.surface.progress != current.surface.progress,
            progress: (!current.surface.progress.is_inactive())
                .then_some(current.surface.progress.clone()),
            line_updates: line_updates(previous, current),
        };

        Self {
            pane_id: current.pane_id,
            from_sequence: previous.sequence,
            to_sequence: current.sequence,
            rows: current.rows,
            cols: current.cols,
            source: current.source,
            buffer_kind: current.buffer_kind,
            patch: (!patch.is_empty()).then_some(patch),
            full_replace: None,
        }
    }
}

fn requires_full_replace(previous: &ScreenSnapshot, current: &ScreenSnapshot) -> bool {
    previous.pane_id != current.pane_id
        || previous.rows != current.rows
        || previous.cols != current.cols
        || previous.source != current.source
        || previous.buffer_kind != current.buffer_kind
}

fn line_updates(previous: &ScreenSnapshot, current: &ScreenSnapshot) -> Vec<ScreenLinePatch> {
    let mut line_updates = Vec::new();
    let total_rows = previous.surface.lines.len().max(current.surface.lines.len());

    for row in 0..total_rows {
        let previous_line = previous.surface.lines.get(row);
        let current_line = current.surface.lines.get(row);
        if previous_line != current_line {
            line_updates.push(ScreenLinePatch {
                row: row as u16,
                line: current_line.cloned().unwrap_or_else(empty_screen_line),
            });
        }
    }

    line_updates
}

fn empty_screen_line() -> ScreenLine {
    ScreenLine::plain(String::new())
}
