use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use terminal_domain::PaneId;
use terminal_projection::{ProjectionSource, ScreenLine, ScreenSnapshot, ScreenSurface};

use crate::snapshot::ZellijPaneTarget;

pub(crate) fn screen_lines_from_output(output: &str) -> Vec<ScreenLine> {
    output.lines().map(|line| ScreenLine { text: line.to_string() }).collect()
}

pub(crate) fn screen_snapshot_from_lines(
    pane_id: PaneId,
    pane_target: &ZellijPaneTarget,
    lines: Vec<ScreenLine>,
    source: ProjectionSource,
) -> ScreenSnapshot {
    let surface = ScreenSurface { title: pane_target.title.clone(), cursor: None, lines };
    let sequence = screen_sequence(
        pane_id,
        pane_target.rows,
        pane_target.cols,
        surface.title.as_deref(),
        &surface.lines,
    );

    ScreenSnapshot {
        pane_id,
        sequence,
        rows: pane_target.rows,
        cols: pane_target.cols,
        source,
        surface,
    }
}

fn screen_sequence(
    pane_id: PaneId,
    rows: u16,
    cols: u16,
    title: Option<&str>,
    lines: &[ScreenLine],
) -> u64 {
    let mut hasher = DefaultHasher::new();
    pane_id.hash(&mut hasher);
    rows.hash(&mut hasher);
    cols.hash(&mut hasher);
    title.hash(&mut hasher);
    for line in lines {
        line.text.hash(&mut hasher);
    }
    hasher.finish()
}
