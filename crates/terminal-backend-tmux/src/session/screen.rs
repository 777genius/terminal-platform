use terminal_projection::{ScreenBufferKind, screen_surface_from_ansi_bytes};

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
            .run_bytes(Some(&self.target), &capture_pane_rich_snapshot_args(&pane_target.target))?;
        let surface = screen_surface_from_ansi_bytes(&output, pane_target.title.clone());
        let sequence = screen_sequence(pane_id, pane_target.rows, pane_target.cols, &surface);

        Ok(ScreenSnapshot {
            pane_id,
            sequence,
            rows: pane_target.rows,
            cols: pane_target.cols,
            source: ProjectionSource::TmuxCapturePane,
            buffer_kind: ScreenBufferKind::Unknown,
            surface,
        })
    }
}

fn capture_pane_rich_snapshot_args(pane_target: &str) -> [&str; 6] {
    ["capture-pane", "-p", "-J", "-e", "-t", pane_target]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_pane_args_preserve_escape_sequences_for_rich_snapshots() {
        assert_eq!(
            capture_pane_rich_snapshot_args("%1"),
            ["capture-pane", "-p", "-J", "-e", "-t", "%1"]
        );
    }
}
