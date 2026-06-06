use terminal_domain::PaneId;
use terminal_mux_domain::SplitDirection;

use super::{
    super::model::{LayoutResizeOutcome, NativePaneLayoutNode, PaneGeometry},
    geometry::{span_to_ratio_bps, target_to_first_span},
};

impl NativePaneLayoutNode {
    pub(in crate::engine) fn resize_target(
        &mut self,
        target: PaneId,
        desired: PaneGeometry,
        rows: u16,
        cols: u16,
    ) -> LayoutResizeOutcome {
        self.resize_target_with_policy(target, desired, rows, cols, true, true)
    }

    fn resize_target_with_policy(
        &mut self,
        target: PaneId,
        desired: PaneGeometry,
        rows: u16,
        cols: u16,
        allow_row_resize: bool,
        allow_col_resize: bool,
    ) -> LayoutResizeOutcome {
        match self {
            Self::Leaf { .. } => LayoutResizeOutcome::default(),
            Self::Split(split) => {
                let mut outcome = LayoutResizeOutcome::default();
                let first_contains = split.first.contains_pane(target);
                let second_contains = split.second.contains_pane(target);
                if !first_contains && !second_contains {
                    return outcome;
                }

                match split.direction {
                    SplitDirection::Vertical if allow_col_resize && cols > 1 => {
                        let desired_first_cols =
                            target_to_first_span(cols, desired.cols, first_contains);
                        let new_ratio = span_to_ratio_bps(desired_first_cols, cols);
                        if split.ratio_bps != new_ratio {
                            split.ratio_bps = new_ratio;
                            outcome.changed = true;
                        }
                        outcome.col_applied = true;
                    }
                    SplitDirection::Horizontal if allow_row_resize && rows > 1 => {
                        let desired_first_rows =
                            target_to_first_span(rows, desired.rows, first_contains);
                        let new_ratio = span_to_ratio_bps(desired_first_rows, rows);
                        if split.ratio_bps != new_ratio {
                            split.ratio_bps = new_ratio;
                            outcome.changed = true;
                        }
                        outcome.row_applied = true;
                    }
                    _ => {}
                }

                let ((first_rows, first_cols), (second_rows, second_cols)) =
                    split.partition(rows, cols);
                let child_allow_row =
                    allow_row_resize && split.direction != SplitDirection::Horizontal;
                let child_allow_col =
                    allow_col_resize && split.direction != SplitDirection::Vertical;
                let nested = if first_contains {
                    split.first.resize_target_with_policy(
                        target,
                        desired,
                        first_rows,
                        first_cols,
                        child_allow_row,
                        child_allow_col,
                    )
                } else {
                    split.second.resize_target_with_policy(
                        target,
                        desired,
                        second_rows,
                        second_cols,
                        child_allow_row,
                        child_allow_col,
                    )
                };
                outcome.merge(nested);
                outcome
            }
        }
    }
}

impl LayoutResizeOutcome {
    fn merge(&mut self, nested: Self) {
        self.changed |= nested.changed;
        self.row_applied |= nested.row_applied;
        self.col_applied |= nested.col_applied;
    }
}
