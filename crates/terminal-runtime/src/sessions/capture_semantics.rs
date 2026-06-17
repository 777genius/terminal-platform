use terminal_projection::ScreenSnapshot;

pub(in crate::sessions) const RENDERED_PLAINTEXT_SNAPSHOT: &str = "rendered_plaintext_snapshot";
pub(in crate::sessions) const RENDERED_SCREEN_SNAPSHOT: &str = "rendered_screen_snapshot";

pub(in crate::sessions) fn rendered_screen_capture_semantics(
    screen: &ScreenSnapshot,
) -> &'static str {
    if screen.surface.lines.iter().any(|line| line.has_rich_content())
        || screen
            .surface
            .cursor
            .as_ref()
            .is_some_and(|cursor| cursor.shape.is_some() || cursor.blinking)
        || !screen.surface.palette.is_empty()
        || screen.surface.working_directory_uri.is_some()
        || !screen.surface.user_variables.is_empty()
        || screen.surface.bell_count != 0
        || !screen.surface.progress.is_inactive()
    {
        RENDERED_SCREEN_SNAPSHOT
    } else {
        RENDERED_PLAINTEXT_SNAPSHOT
    }
}

#[cfg(test)]
mod tests {
    use terminal_domain::PaneId;
    use terminal_projection::{
        ProjectionSource, ScreenBufferKind, ScreenColor, ScreenCursor, ScreenLine, ScreenLineSpan,
        ScreenSnapshot, ScreenSurface, ScreenSurfacePalette, ScreenTextStyle,
    };

    use super::*;

    #[test]
    fn classifies_plain_screen_snapshots_conservatively() {
        let screen = screen_with_lines(vec![ScreenLine::plain("ready")]);

        assert_eq!(rendered_screen_capture_semantics(&screen), RENDERED_PLAINTEXT_SNAPSHOT);
    }

    #[test]
    fn classifies_styled_screen_snapshots_as_structured_screen() {
        let screen = screen_with_lines(vec![ScreenLine {
            text: "ready".to_string(),
            spans: vec![ScreenLineSpan {
                text: "ready".to_string(),
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

        assert_eq!(rendered_screen_capture_semantics(&screen), RENDERED_SCREEN_SNAPSHOT);
    }

    #[test]
    fn classifies_palette_snapshots_as_structured_screen() {
        let mut screen = screen_with_lines(vec![ScreenLine::plain("ready")]);
        screen.surface.palette = ScreenSurfacePalette {
            foreground: Some(ScreenColor::Rgb { r: 255, g: 255, b: 255 }),
            background: None,
            cursor: None,
        };

        assert_eq!(rendered_screen_capture_semantics(&screen), RENDERED_SCREEN_SNAPSHOT);
    }

    fn screen_with_lines(lines: Vec<ScreenLine>) -> ScreenSnapshot {
        ScreenSnapshot {
            pane_id: PaneId::new(),
            sequence: 1,
            rows: 24,
            cols: 80,
            source: ProjectionSource::NativeEmulator,
            buffer_kind: ScreenBufferKind::Normal,
            surface: ScreenSurface {
                title: None,
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
}
