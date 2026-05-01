use std::collections::HashSet;

use terminal_backend_api::BackendError;
use terminal_domain::PaneId;
use terminal_mux_domain::{PaneSplit, PaneTreeNode, SplitDirection};

use super::{
    DEFAULT_SPLIT_RATIO_BPS, SPLIT_RATIO_SCALE,
    model::{
        LayoutResizeOutcome, NativePaneLayoutNode, NativePaneLayoutSplit, NativeSessionState,
        NativeTabRuntime, PaneGeometry,
    },
};

impl NativeTabRuntime {
    pub(super) fn pane(&self, pane_id: PaneId) -> Option<&super::model::NativePaneRuntime> {
        self.panes.iter().find(|pane| pane.pane_id == pane_id)
    }

    pub(super) fn pane_ids(&self) -> Vec<PaneId> {
        self.root.pane_ids()
    }

    pub(super) fn contains_pane(&self, pane_id: PaneId) -> bool {
        self.root.contains_pane(pane_id)
    }

    pub(super) fn first_pane_id(&self) -> Option<PaneId> {
        self.root.first_pane_id()
    }
}

impl NativePaneLayoutNode {
    pub(super) fn from_snapshot(root: PaneTreeNode) -> Self {
        match root {
            PaneTreeNode::Leaf { pane_id } => Self::Leaf { pane_id },
            PaneTreeNode::Split(split) => Self::Split(NativePaneLayoutSplit {
                direction: split.direction,
                ratio_bps: DEFAULT_SPLIT_RATIO_BPS,
                first: Box::new(Self::from_snapshot(*split.first)),
                second: Box::new(Self::from_snapshot(*split.second)),
            }),
        }
    }

    pub(super) fn snapshot(&self) -> PaneTreeNode {
        match self {
            Self::Leaf { pane_id } => PaneTreeNode::Leaf { pane_id: *pane_id },
            Self::Split(split) => PaneTreeNode::Split(PaneSplit {
                direction: split.direction,
                first: Box::new(split.first.snapshot()),
                second: Box::new(split.second.snapshot()),
            }),
        }
    }

    pub(super) fn contains_pane(&self, target: PaneId) -> bool {
        match self {
            Self::Leaf { pane_id } => *pane_id == target,
            Self::Split(split) => {
                split.first.contains_pane(target) || split.second.contains_pane(target)
            }
        }
    }

    pub(super) fn pane_ids(&self) -> Vec<PaneId> {
        let mut pane_ids = Vec::new();
        self.collect_pane_ids(&mut pane_ids);
        pane_ids
    }

    pub(super) fn path_has_direction(&self, target: PaneId, direction: SplitDirection) -> bool {
        match self {
            Self::Leaf { .. } => false,
            Self::Split(split) => {
                let first_contains = split.first.contains_pane(target);
                let second_contains = split.second.contains_pane(target);
                if !first_contains && !second_contains {
                    return false;
                }

                if split.direction == direction {
                    true
                } else if first_contains {
                    split.first.path_has_direction(target, direction)
                } else {
                    split.second.path_has_direction(target, direction)
                }
            }
        }
    }

    pub(super) fn first_pane_id(&self) -> Option<PaneId> {
        match self {
            Self::Leaf { pane_id } => Some(*pane_id),
            Self::Split(split) => {
                split.first.first_pane_id().or_else(|| split.second.first_pane_id())
            }
        }
    }

    pub(super) fn split_leaf(
        &mut self,
        target: PaneId,
        direction: SplitDirection,
        new_pane: PaneId,
    ) -> bool {
        match self {
            Self::Leaf { pane_id } if *pane_id == target => {
                let current_pane = *pane_id;
                *self = Self::Split(NativePaneLayoutSplit {
                    direction,
                    ratio_bps: DEFAULT_SPLIT_RATIO_BPS,
                    first: Box::new(Self::Leaf { pane_id: current_pane }),
                    second: Box::new(Self::Leaf { pane_id: new_pane }),
                });
                true
            }
            Self::Leaf { .. } => false,
            Self::Split(split) => {
                split.first.split_leaf(target, direction, new_pane)
                    || split.second.split_leaf(target, direction, new_pane)
            }
        }
    }

    pub(super) fn remove_leaf(&self, target: PaneId) -> Option<Self> {
        match self {
            Self::Leaf { pane_id } => {
                (*pane_id != target).then_some(Self::Leaf { pane_id: *pane_id })
            }
            Self::Split(split) => {
                match (split.first.remove_leaf(target), split.second.remove_leaf(target)) {
                    (Some(first), Some(second)) => Some(Self::Split(NativePaneLayoutSplit {
                        direction: split.direction,
                        ratio_bps: split.ratio_bps,
                        first: Box::new(first),
                        second: Box::new(second),
                    })),
                    (Some(node), None) | (None, Some(node)) => Some(node),
                    (None, None) => None,
                }
            }
        }
    }

    pub(super) fn resize_target(
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

    fn collect_pane_ids(&self, pane_ids: &mut Vec<PaneId>) {
        match self {
            Self::Leaf { pane_id } => pane_ids.push(*pane_id),
            Self::Split(split) => {
                split.first.collect_pane_ids(pane_ids);
                split.second.collect_pane_ids(pane_ids);
            }
        }
    }
}

impl NativePaneLayoutSplit {
    fn partition(&self, rows: u16, cols: u16) -> ((u16, u16), (u16, u16)) {
        match self.direction {
            SplitDirection::Vertical => {
                let (first_cols, second_cols) = partition_dimension_by_ratio(cols, self.ratio_bps);
                ((rows, first_cols), (rows, second_cols))
            }
            SplitDirection::Horizontal => {
                let (first_rows, second_rows) = partition_dimension_by_ratio(rows, self.ratio_bps);
                ((first_rows, cols), (second_rows, cols))
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

pub(super) fn reflow_tab_layout(
    tab: &NativeTabRuntime,
    rows: u16,
    cols: u16,
) -> Result<(), BackendError> {
    apply_pane_layout(&tab.root, tab, rows.max(1), cols.max(1))
}

pub(super) fn collect_surface_updates(state: &NativeSessionState, pane_id: PaneId) -> Vec<PaneId> {
    state
        .tabs
        .iter()
        .find(|tab| tab.contains_pane(pane_id))
        .map_or_else(Vec::new, NativeTabRuntime::pane_ids)
}

fn apply_pane_layout(
    node: &NativePaneLayoutNode,
    tab: &NativeTabRuntime,
    rows: u16,
    cols: u16,
) -> Result<(), BackendError> {
    match node {
        NativePaneLayoutNode::Leaf { pane_id } => {
            let pane = tab.pane(*pane_id).ok_or_else(|| {
                BackendError::internal(format!(
                    "native pane tree references missing pane {pane_id:?}"
                ))
            })?;
            pane.resize(rows, cols)?;
            Ok(())
        }
        NativePaneLayoutNode::Split(split) => {
            let ((first_rows, first_cols), (second_rows, second_cols)) =
                split.partition(rows, cols);
            apply_pane_layout(&split.first, tab, first_rows, first_cols)?;
            apply_pane_layout(&split.second, tab, second_rows, second_cols)
        }
    }
}

pub(super) fn validate_layout_override(
    tab: &NativeTabRuntime,
    root: &PaneTreeNode,
) -> Result<(), BackendError> {
    let current_panes: HashSet<_> = tab.pane_ids().into_iter().collect();
    let requested_panes = collect_snapshot_pane_ids(root);
    let requested_unique: HashSet<_> = requested_panes.iter().copied().collect();

    if requested_panes.len() != requested_unique.len() {
        return Err(BackendError::invalid_input("layout override contains duplicate pane ids"));
    }
    if current_panes != requested_unique {
        return Err(BackendError::invalid_input(
            "layout override must preserve the exact pane set for the target tab",
        ));
    }

    Ok(())
}

fn collect_snapshot_pane_ids(root: &PaneTreeNode) -> Vec<PaneId> {
    let mut pane_ids = Vec::new();
    collect_snapshot_pane_ids_inner(root, &mut pane_ids);
    pane_ids
}

fn collect_snapshot_pane_ids_inner(root: &PaneTreeNode, pane_ids: &mut Vec<PaneId>) {
    match root {
        PaneTreeNode::Leaf { pane_id } => pane_ids.push(*pane_id),
        PaneTreeNode::Split(split) => {
            collect_snapshot_pane_ids_inner(&split.first, pane_ids);
            collect_snapshot_pane_ids_inner(&split.second, pane_ids);
        }
    }
}

fn target_to_first_span(total: u16, desired_target: u16, target_is_first: bool) -> u16 {
    if total <= 1 {
        return 1;
    }

    let clamped_target = desired_target.clamp(1, total.saturating_sub(1));
    if target_is_first {
        clamped_target
    } else {
        total.saturating_sub(clamped_target).clamp(1, total.saturating_sub(1))
    }
}

fn span_to_ratio_bps(first_span: u16, total: u16) -> u16 {
    if total <= 1 {
        return DEFAULT_SPLIT_RATIO_BPS;
    }

    let clamped_first = first_span.clamp(1, total.saturating_sub(1));
    let ratio = ((u32::from(clamped_first) * u32::from(SPLIT_RATIO_SCALE))
        + (u32::from(total) / 2))
        / u32::from(total);
    ratio
        .clamp(1, u32::from(SPLIT_RATIO_SCALE.saturating_sub(1)))
        .try_into()
        .unwrap_or(DEFAULT_SPLIT_RATIO_BPS)
}

fn partition_dimension_by_ratio(total: u16, ratio_bps: u16) -> (u16, u16) {
    if total <= 1 {
        return (1, 1);
    }

    let ratio = ratio_bps.clamp(1, SPLIT_RATIO_SCALE.saturating_sub(1));
    let mut first = ((u32::from(total) * u32::from(ratio)) + (u32::from(SPLIT_RATIO_SCALE) / 2))
        / u32::from(SPLIT_RATIO_SCALE);
    first = first.clamp(1, u32::from(total.saturating_sub(1)));
    let first: u16 = first.try_into().unwrap_or(1);
    let second = total.saturating_sub(first).max(1);
    (first, second)
}
