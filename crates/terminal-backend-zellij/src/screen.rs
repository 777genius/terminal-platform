use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use terminal_domain::PaneId;
use terminal_projection::{
    ProjectionSource, ScreenBufferKind, ScreenSnapshot, ScreenSurface,
    screen_surface_from_ansi_bytes, screen_surface_from_ansi_output,
};
#[cfg(test)]
use terminal_projection::{
    ScreenLine, screen_lines_from_ansi_bytes, screen_lines_from_ansi_output,
};

use crate::snapshot::ZellijPaneTarget;

#[cfg(test)]
pub(crate) fn screen_lines_from_output(output: &str) -> Vec<ScreenLine> {
    screen_lines_from_ansi_output(output)
}

#[cfg(test)]
pub(crate) fn screen_lines_from_bytes(output: &[u8]) -> Vec<ScreenLine> {
    screen_lines_from_ansi_bytes(output)
}

pub(crate) fn screen_surface_from_output(output: &str, title: Option<String>) -> ScreenSurface {
    screen_surface_from_ansi_output(output, title)
}

pub(crate) fn screen_surface_from_bytes(output: &[u8], title: Option<String>) -> ScreenSurface {
    screen_surface_from_ansi_bytes(output, title)
}

pub(crate) fn screen_snapshot_from_surface(
    pane_id: PaneId,
    pane_target: &ZellijPaneTarget,
    surface: ScreenSurface,
    source: ProjectionSource,
) -> ScreenSnapshot {
    let sequence = screen_sequence(pane_id, pane_target.rows, pane_target.cols, &surface);

    ScreenSnapshot {
        pane_id,
        sequence,
        rows: pane_target.rows,
        cols: pane_target.cols,
        source,
        buffer_kind: ScreenBufferKind::Unknown,
        surface,
    }
}

fn screen_sequence(pane_id: PaneId, rows: u16, cols: u16, surface: &ScreenSurface) -> u64 {
    let mut hasher = DefaultHasher::new();
    pane_id.hash(&mut hasher);
    rows.hash(&mut hasher);
    cols.hash(&mut hasher);
    surface.title.hash(&mut hasher);
    surface.working_directory_uri.hash(&mut hasher);
    surface.user_variables.hash(&mut hasher);
    surface.cursor.hash(&mut hasher);
    surface.palette.hash(&mut hasher);
    surface.bell_count.hash(&mut hasher);
    surface.progress.hash(&mut hasher);
    for line in &surface.lines {
        line.hash(&mut hasher);
    }
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use terminal_projection::{
        ScreenColor, ScreenLineSpan, ScreenProgress, ScreenProgressState, ScreenTextStyle,
    };

    use super::*;

    #[test]
    fn screen_sequence_changes_when_line_style_changes() {
        let pane_id = PaneId::new();
        let plain = surface(vec![ScreenLine::plain("status")]);
        let styled = surface(vec![ScreenLine {
            text: "status".to_string(),
            spans: vec![ScreenLineSpan {
                text: "status".to_string(),
                style: ScreenTextStyle {
                    foreground: Some(ScreenColor::Named { name: "red".to_string() }),
                    ..ScreenTextStyle::default()
                },
            }],
            media: Vec::new(),
            side_effects: Vec::new(),
            semantic_marks: Vec::new(),
            wrapped: false,
        }]);

        assert_ne!(
            screen_sequence(pane_id, 24, 80, &plain),
            screen_sequence(pane_id, 24, 80, &styled)
        );
    }

    #[test]
    fn screen_sequence_changes_when_surface_metadata_changes() {
        let pane_id = PaneId::new();
        let plain = surface(vec![ScreenLine::plain("status")]);
        let mut progress = plain.clone();
        progress.progress = ScreenProgress { state: ScreenProgressState::Normal, value: Some(42) };

        assert_ne!(
            screen_sequence(pane_id, 24, 80, &plain),
            screen_sequence(pane_id, 24, 80, &progress)
        );
    }

    #[test]
    fn screen_lines_from_output_preserves_ansi_rich_spans() {
        let lines = screen_lines_from_output("\x1b[32mok\x1b[0m\nplain");

        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].text, "ok");
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "ok"
                && span.style.foreground == Some(ScreenColor::Named { name: "green".to_string() })
        }));
        assert_eq!(lines[1], ScreenLine::plain("plain"));
    }

    #[test]
    fn screen_lines_from_bytes_preserves_raw_c1_controls() {
        let lines = screen_lines_from_bytes(b"\x9b32mok\x9b0m");

        assert_eq!(lines[0].text, "ok");
        assert!(lines[0].spans.iter().any(|span| {
            span.text == "ok"
                && span.style.foreground == Some(ScreenColor::Named { name: "green".to_string() })
        }));
    }

    #[test]
    fn screen_surface_from_bytes_preserves_terminal_metadata() {
        let surface = screen_surface_from_bytes(
            b"\x1b]2;Build\x07\x1b]7;file://localhost/tmp/project\x07\x1b]9;4;1;42\x07ok",
            Some("fallback".to_string()),
        );

        assert_eq!(surface.title.as_deref(), Some("Build"));
        assert_eq!(surface.working_directory_uri.as_deref(), Some("file://localhost/tmp/project"));
        assert_eq!(
            surface.progress,
            ScreenProgress { state: ScreenProgressState::Normal, value: Some(42) }
        );
        assert_eq!(surface.lines, vec![ScreenLine::plain("ok")]);
    }

    fn surface(lines: Vec<ScreenLine>) -> ScreenSurface {
        ScreenSurface {
            title: Some("pane".to_string()),
            working_directory_uri: None,
            user_variables: Default::default(),
            cursor: None,
            palette: Default::default(),
            bell_count: 0,
            progress: Default::default(),
            lines,
        }
    }
}
