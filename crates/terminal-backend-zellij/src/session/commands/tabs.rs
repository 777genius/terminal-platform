use terminal_backend_api::{BackendError, NewTabSpec};
use terminal_domain::{DegradedModeReason, TabId};

use crate::{action::ZellijAction, snapshot::ZellijSessionSnapshot};

use super::super::ZellijAttachedSession;

impl ZellijAttachedSession {
    pub(super) fn new_tab_actions(&self, spec: NewTabSpec) -> Vec<ZellijAction> {
        vec![ZellijAction::NewTab { title: spec.title }]
    }

    pub(super) fn focus_tab_action(
        &self,
        snapshot: &ZellijSessionSnapshot,
        tab_id: TabId,
    ) -> Result<ZellijAction, BackendError> {
        let tab_target = snapshot
            .tab_targets
            .get(&tab_id)
            .cloned()
            .ok_or_else(|| BackendError::not_found(format!("unknown zellij tab {tab_id:?}")))?;
        Ok(ZellijAction::FocusTab {
            backend_tab_id: tab_target.backend_tab_id,
            display_index: tab_target.display_index,
        })
    }

    pub(super) fn close_tab_action(
        &self,
        snapshot: &ZellijSessionSnapshot,
        tab_id: TabId,
    ) -> Result<ZellijAction, BackendError> {
        if snapshot.topology.tabs.len() <= 1 {
            return Err(BackendError::unsupported(
                "zellij imported routes refuse to close the last tab because it would terminate the foreign session",
                DegradedModeReason::UnsupportedByBackend,
            ));
        }
        let tab_target = snapshot
            .tab_targets
            .get(&tab_id)
            .cloned()
            .ok_or_else(|| BackendError::not_found(format!("unknown zellij tab {tab_id:?}")))?;

        Ok(ZellijAction::CloseTab { backend_tab_id: tab_target.backend_tab_id })
    }

    pub(super) fn rename_tab_action(
        &self,
        snapshot: &ZellijSessionSnapshot,
        tab_id: TabId,
        title: &str,
    ) -> Result<ZellijAction, BackendError> {
        let tab_target = snapshot
            .tab_targets
            .get(&tab_id)
            .cloned()
            .ok_or_else(|| BackendError::not_found(format!("unknown zellij tab {tab_id:?}")))?;
        Ok(ZellijAction::RenameTab {
            backend_tab_id: tab_target.backend_tab_id,
            title: title.to_string(),
        })
    }
}
