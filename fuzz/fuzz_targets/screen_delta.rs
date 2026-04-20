#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use terminal_domain::PaneId;
use terminal_projection::{
    ProjectionSource, ScreenCursor, ScreenDelta, ScreenLine, ScreenSnapshot, ScreenSurface,
};
use uuid::Uuid;

#[derive(Arbitrary, Debug)]
struct SnapshotInput {
    pane_seed: [u8; 16],
    previous_sequence: u64,
    current_sequence: u64,
    previous_rows: u16,
    previous_cols: u16,
    current_rows: u16,
    current_cols: u16,
    previous_title: Option<String>,
    current_title: Option<String>,
    previous_lines: Vec<String>,
    current_lines: Vec<String>,
    previous_cursor: Option<(u16, u16)>,
    current_cursor: Option<(u16, u16)>,
    previous_source: u8,
    current_source: u8,
}

fn projection_source(seed: u8) -> ProjectionSource {
    match seed % 6 {
        0 => ProjectionSource::NativeEmulator,
        1 => ProjectionSource::NativeTranscript,
        2 => ProjectionSource::TmuxCapturePane,
        3 => ProjectionSource::TmuxRawOutputImport,
        4 => ProjectionSource::ZellijViewportSubscribe,
        _ => ProjectionSource::ZellijDumpSnapshot,
    }
}

fn bounded_lines(lines: Vec<String>) -> Vec<ScreenLine> {
    lines
        .into_iter()
        .take(64)
        .map(|text| ScreenLine {
            text: text.chars().take(256).collect(),
        })
        .collect()
}

fn bounded_title(title: Option<String>) -> Option<String> {
    title.map(|value| value.chars().take(128).collect())
}

fn bounded_cursor(cursor: Option<(u16, u16)>) -> Option<ScreenCursor> {
    cursor.map(|(row, col)| ScreenCursor { row, col })
}

fn snapshot(
    pane_id: PaneId,
    sequence: u64,
    rows: u16,
    cols: u16,
    source: ProjectionSource,
    title: Option<String>,
    lines: Vec<String>,
    cursor: Option<(u16, u16)>,
) -> ScreenSnapshot {
    ScreenSnapshot {
        pane_id,
        sequence,
        rows: rows.max(1),
        cols: cols.max(1),
        source,
        surface: ScreenSurface {
            title: bounded_title(title),
            cursor: bounded_cursor(cursor),
            lines: bounded_lines(lines),
        },
    }
}

fuzz_target!(|input: SnapshotInput| {
    let pane_id = PaneId::from(Uuid::from_bytes(input.pane_seed));
    let previous = snapshot(
        pane_id,
        input.previous_sequence,
        input.previous_rows,
        input.previous_cols,
        projection_source(input.previous_source),
        input.previous_title,
        input.previous_lines,
        input.previous_cursor,
    );
    let current = snapshot(
        pane_id,
        input.current_sequence,
        input.current_rows,
        input.current_cols,
        projection_source(input.current_source),
        input.current_title,
        input.current_lines,
        input.current_cursor,
    );

    let _ = ScreenDelta::between(&previous, &current);
    let _ = ScreenDelta::unchanged_from(&current);
    let _ = ScreenDelta::full_replace(previous.sequence, &current);
});
