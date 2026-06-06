use super::super::TmuxAttachedSession;
use crate::{prelude::*, util::*};

impl TmuxAttachedSession {
    pub(super) fn focus_pane(&self, pane_id: PaneId) -> Result<MuxCommandResult, BackendError> {
        let snapshot = self.snapshot()?;
        let pane_target = snapshot
            .pane_targets
            .get(&pane_id)
            .ok_or_else(|| BackendError::not_found(format!("unknown tmux pane {pane_id:?}")))?;
        self.backend.run(Some(&self.target), &["select-pane", "-t", &pane_target.target])?;

        Ok(MuxCommandResult { changed: true })
    }

    pub(super) fn split_pane(&self, spec: SplitPaneSpec) -> Result<MuxCommandResult, BackendError> {
        let snapshot = self.snapshot()?;
        let pane_target = snapshot.pane_targets.get(&spec.pane_id).ok_or_else(|| {
            BackendError::not_found(format!("unknown tmux pane {:?}", spec.pane_id))
        })?;
        self.backend.run(
            Some(&self.target),
            &["split-window", tmux_split_flag(spec.direction), "-t", &pane_target.target],
        )?;

        Ok(MuxCommandResult { changed: true })
    }

    pub(super) fn close_pane(&self, pane_id: PaneId) -> Result<MuxCommandResult, BackendError> {
        let snapshot = self.snapshot()?;
        let pane_target = snapshot
            .pane_targets
            .get(&pane_id)
            .ok_or_else(|| BackendError::not_found(format!("unknown tmux pane {pane_id:?}")))?;
        let tab =
            snapshot.topology.tabs.iter().find(|tab| tab_contains_pane(tab, pane_id)).ok_or_else(
                || BackendError::not_found(format!("tmux pane {pane_id:?} is not bound to a tab")),
            )?;
        if collect_pane_ids(&tab.root).len() <= 1 {
            return Err(BackendError::unsupported(
                "tmux imported routes refuse to close the last pane in a tab because it would collapse tab lifecycle into tab closure semantics",
                DegradedModeReason::UnsupportedByBackend,
            ));
        }
        self.backend.run(Some(&self.target), &["kill-pane", "-t", &pane_target.target])?;

        Ok(MuxCommandResult { changed: true })
    }

    pub(super) fn resize_pane(
        &self,
        spec: ResizePaneSpec,
    ) -> Result<MuxCommandResult, BackendError> {
        let snapshot = self.snapshot()?;
        let pane_target = snapshot.pane_targets.get(&spec.pane_id).ok_or_else(|| {
            BackendError::not_found(format!("unknown tmux pane {:?}", spec.pane_id))
        })?;
        if pane_target.rows == spec.rows && pane_target.cols == spec.cols {
            return Ok(MuxCommandResult { changed: false });
        }
        let rows = spec.rows.to_string();
        let cols = spec.cols.to_string();
        self.backend.run(
            Some(&self.target),
            &["resize-pane", "-t", &pane_target.target, "-y", &rows, "-x", &cols],
        )?;

        Ok(MuxCommandResult { changed: true })
    }
}
