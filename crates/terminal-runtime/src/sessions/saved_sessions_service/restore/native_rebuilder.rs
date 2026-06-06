use std::collections::HashMap;

use terminal_backend_api::{BackendError, MuxCommand, NewTabSpec};
use terminal_domain::{PaneId, SessionId, TabId};
use terminal_mux_domain::TabSnapshot;
use terminal_persistence::SavedNativeSession;

use crate::sessions::{active_session_service::ActiveSessionService, runtime::SessionRuntime};

use super::layout_rebuilder::rebuild_saved_tab_layout;

pub(super) struct SavedNativeSessionRebuilder<'a> {
    active: ActiveSessionService<'a>,
}

impl<'a> SavedNativeSessionRebuilder<'a> {
    pub(super) fn new(runtime: SessionRuntime<'a>) -> Self {
        Self { active: ActiveSessionService::new(runtime) }
    }

    pub(super) async fn rebuild(
        &self,
        restored_session_id: SessionId,
        saved: &SavedNativeSession,
    ) -> Result<(), BackendError> {
        self.create_missing_tabs(restored_session_id, saved).await?;

        let topology = self.active.topology_snapshot(restored_session_id).await?;
        if topology.tabs.len() != saved.topology.tabs.len() {
            return Err(BackendError::internal(format!(
                "restored native session tab count drifted during rebuild - live {} saved {}",
                topology.tabs.len(),
                saved.topology.tabs.len()
            )));
        }

        let mut restored_focus_tab_id = None;
        for (index, saved_tab) in saved.topology.tabs.iter().enumerate() {
            let live_tab = topology.tabs.get(index).ok_or_else(|| {
                BackendError::internal("restored native session lost live tab during rebuild")
            })?;
            let live_tab_id = live_tab.tab_id;
            self.restore_tab(restored_session_id, live_tab_id, live_tab, saved_tab).await?;

            if saved.topology.focused_tab == Some(saved_tab.tab_id) {
                restored_focus_tab_id = Some(live_tab_id);
            }
        }

        self.focus_restored_tab(restored_session_id, restored_focus_tab_id).await
    }

    async fn create_missing_tabs(
        &self,
        restored_session_id: SessionId,
        saved: &SavedNativeSession,
    ) -> Result<(), BackendError> {
        for saved_tab in saved.topology.tabs.iter().skip(1) {
            self.active
                .dispatch(
                    restored_session_id,
                    MuxCommand::NewTab(NewTabSpec { title: saved_tab.title.clone() }),
                )
                .await?;
        }

        Ok(())
    }

    async fn restore_tab(
        &self,
        restored_session_id: SessionId,
        live_tab_id: TabId,
        live_tab: &TabSnapshot,
        saved_tab: &TabSnapshot,
    ) -> Result<(), BackendError> {
        self.rename_tab_if_needed(restored_session_id, live_tab_id, live_tab, saved_tab).await?;

        let pane_map =
            rebuild_saved_tab_layout(&self.active, restored_session_id, live_tab_id, saved_tab)
                .await?;
        self.focus_saved_pane(restored_session_id, saved_tab, &pane_map).await
    }

    async fn rename_tab_if_needed(
        &self,
        restored_session_id: SessionId,
        live_tab_id: TabId,
        live_tab: &TabSnapshot,
        saved_tab: &TabSnapshot,
    ) -> Result<(), BackendError> {
        if let Some(saved_title) = &saved_tab.title
            && live_tab.title.as_deref() != Some(saved_title.as_str())
        {
            self.active
                .dispatch(
                    restored_session_id,
                    MuxCommand::RenameTab { tab_id: live_tab_id, title: saved_title.clone() },
                )
                .await?;
        }

        Ok(())
    }

    async fn focus_saved_pane(
        &self,
        restored_session_id: SessionId,
        saved_tab: &TabSnapshot,
        pane_map: &HashMap<PaneId, PaneId>,
    ) -> Result<(), BackendError> {
        if let Some(saved_focused_pane) = saved_tab.focused_pane
            && let Some(restored_pane_id) = pane_map.get(&saved_focused_pane).copied()
        {
            self.active
                .dispatch(restored_session_id, MuxCommand::FocusPane { pane_id: restored_pane_id })
                .await?;
        }

        Ok(())
    }

    async fn focus_restored_tab(
        &self,
        restored_session_id: SessionId,
        restored_focus_tab_id: Option<TabId>,
    ) -> Result<(), BackendError> {
        if let Some(restored_focus_tab_id) = restored_focus_tab_id {
            self.active
                .dispatch(
                    restored_session_id,
                    MuxCommand::FocusTab { tab_id: restored_focus_tab_id },
                )
                .await?;
        }

        Ok(())
    }
}
