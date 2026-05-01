use super::TmuxAttachedSession;
use crate::{prelude::*, sequence::screen_sequence};

impl TmuxAttachedSession {
    pub(super) fn screen_snapshot_inner(
        &self,
        pane_id: PaneId,
    ) -> Result<ScreenSnapshot, BackendError> {
        let snapshot = self.snapshot()?;
        let pane_target = snapshot
            .pane_targets
            .get(&pane_id)
            .ok_or_else(|| BackendError::not_found(format!("unknown tmux pane {pane_id:?}")))?;
        let output = self
            .backend
            .run(Some(&self.target), &["capture-pane", "-p", "-J", "-t", &pane_target.target])?;
        let lines: Vec<ScreenLine> =
            output.lines().map(|line| ScreenLine { text: line.to_string() }).collect();
        let surface = ScreenSurface { title: pane_target.title.clone(), cursor: None, lines };
        let sequence = screen_sequence(
            pane_id,
            pane_target.rows,
            pane_target.cols,
            surface.title.as_deref(),
            &surface.lines,
        );

        Ok(ScreenSnapshot {
            pane_id,
            sequence,
            rows: pane_target.rows,
            cols: pane_target.cols,
            source: ProjectionSource::TmuxCapturePane,
            surface,
        })
    }
}
