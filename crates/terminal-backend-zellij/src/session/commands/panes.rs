use terminal_backend_api::BackendError;
use terminal_domain::{DegradedModeReason, PaneId};

use crate::{
    action::ZellijAction,
    snapshot::{ZellijSessionSnapshot, collect_pane_ids, tab_contains_pane},
};

use super::super::ZellijAttachedSession;

impl ZellijAttachedSession {
    pub(super) fn focus_pane_action(
        &self,
        snapshot: &ZellijSessionSnapshot,
        pane_id: PaneId,
    ) -> Result<ZellijAction, BackendError> {
        let pane_target =
            snapshot.pane_targets.get(&pane_id).cloned().ok_or_else(|| {
                BackendError::not_found(format!("unknown zellij pane {pane_id:?}"))
            })?;
        Ok(ZellijAction::FocusPane { pane_ref: pane_target.backend_ref })
    }

    pub(super) fn close_pane_action(
        &self,
        snapshot: &ZellijSessionSnapshot,
        pane_id: PaneId,
    ) -> Result<ZellijAction, BackendError> {
        let pane_target =
            snapshot.pane_targets.get(&pane_id).cloned().ok_or_else(|| {
                BackendError::not_found(format!("unknown zellij pane {pane_id:?}"))
            })?;
        let tab = snapshot
            .topology
            .tabs
            .iter()
            .find(|tab| tab_contains_pane(tab, pane_id))
            .ok_or_else(|| {
                BackendError::not_found(format!("zellij pane {pane_id:?} is not bound to a tab"))
            })?;
        if collect_pane_ids(&tab.root).len() <= 1 {
            return Err(BackendError::unsupported(
                "zellij imported routes refuse to close the last pane in a tab because it would collapse tab lifecycle into tab closure semantics",
                DegradedModeReason::UnsupportedByBackend,
            ));
        }

        Ok(ZellijAction::ClosePane { pane_ref: pane_target.backend_ref })
    }
}
