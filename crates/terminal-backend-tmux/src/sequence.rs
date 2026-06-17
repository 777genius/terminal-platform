use crate::prelude::*;

pub(crate) fn screen_sequence(
    pane_id: PaneId,
    rows: u16,
    cols: u16,
    surface: &ScreenSurface,
) -> u64 {
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
        ScreenColor, ScreenLine, ScreenLineSpan, ScreenProgress, ScreenProgressState,
        ScreenSurface, ScreenTextStyle,
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
        let mut plain = surface(vec![ScreenLine::plain("status")]);
        let mut progress = plain.clone();
        progress.progress = ScreenProgress { state: ScreenProgressState::Normal, value: Some(42) };

        assert_ne!(
            screen_sequence(pane_id, 24, 80, &plain),
            screen_sequence(pane_id, 24, 80, &progress)
        );

        plain.working_directory_uri = Some("file://localhost/tmp/project".to_string());
        assert_ne!(
            screen_sequence(pane_id, 24, 80, &plain),
            screen_sequence(pane_id, 24, 80, &progress)
        );
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
