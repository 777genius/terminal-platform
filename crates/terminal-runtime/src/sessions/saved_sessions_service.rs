use std::collections::HashMap;

use terminal_backend_api::{
    BackendError, BackendSessionSummary, MuxCommand, MuxCommandResult, NewTabSpec, SplitPaneSpec,
};
use terminal_domain::{
    BackendKind, DegradedModeReason, PaneId, SavedSessionManifest, SessionId, TabId,
    saved_session_compatibility,
};
use terminal_mux_domain::{PaneTreeNode, TabSnapshot};
use terminal_persistence::{
    PrunedSavedSessions, SavedNativeSession, SavedSessionSummary as PersistedSavedSessionSummary,
    SqliteSessionStore,
};

use super::{
    active_session_service::ActiveSessionService,
    runtime::{
        SessionRuntime, collect_pane_ids_from_node, collect_pane_ids_from_topology,
        saved_session_title, tab_snapshot_by_id,
    },
};

#[derive(Clone, Copy)]
pub(super) struct SavedSessionsService<'a> {
    runtime: SessionRuntime<'a>,
}

impl<'a> SavedSessionsService<'a> {
    pub(super) fn new(runtime: SessionRuntime<'a>) -> Self {
        Self { runtime }
    }

    pub(super) fn list_saved_sessions(
        &self,
    ) -> Result<Vec<PersistedSavedSessionSummary>, BackendError> {
        self.runtime.persistence().list_native_sessions().map_err(|error| {
            BackendError::internal(format!("failed to list saved native sessions - {error}"))
        })
    }

    pub(super) fn saved_session(
        &self,
        session_id: SessionId,
    ) -> Result<SavedNativeSession, BackendError> {
        self.runtime
            .persistence()
            .load_native_session(session_id)
            .map_err(|error| {
                BackendError::internal(format!("failed to load saved native session - {error}"))
            })?
            .ok_or_else(|| BackendError::not_found(format!("unknown saved session {session_id:?}")))
    }

    pub(super) fn delete_saved_session(&self, session_id: SessionId) -> Result<(), BackendError> {
        let deleted =
            self.runtime.persistence().delete_native_session(session_id).map_err(|error| {
                BackendError::internal(format!("failed to delete saved native session - {error}"))
            })?;
        if !deleted {
            return Err(BackendError::not_found(format!("unknown saved session {session_id:?}")));
        }

        Ok(())
    }

    pub(super) fn prune_saved_sessions(
        &self,
        keep_latest: usize,
    ) -> Result<PrunedSavedSessions, BackendError> {
        self.runtime.persistence().prune_native_sessions(keep_latest).map_err(|error| {
            BackendError::internal(format!("failed to prune saved native sessions - {error}"))
        })
    }

    pub(super) async fn restore_saved_session(
        &self,
        session_id: SessionId,
    ) -> Result<BackendSessionSummary, BackendError> {
        let saved = self.saved_session(session_id)?;
        let compatibility = saved_session_compatibility(&saved.manifest);
        if !compatibility.can_restore {
            return Err(BackendError::unsupported(
                format!(
                    "saved session manifest is not restore-compatible - {:?}",
                    compatibility.status
                ),
                DegradedModeReason::SavedSessionIncompatible,
            ));
        }
        if saved.route.backend != BackendKind::Native {
            return Err(BackendError::unsupported(
                "restore saved session is only implemented for native sessions in v1",
                DegradedModeReason::UnsupportedByBackend,
            ));
        }
        if saved.topology.tabs.is_empty() {
            return Err(BackendError::invalid_input("saved native session has no tabs"));
        }

        let initial_title =
            saved.topology.tabs.first().and_then(|tab| tab.title.clone()).or(saved.title.clone());
        let restored = self
            .runtime
            .create_native_session(terminal_backend_api::CreateSessionSpec {
                title: initial_title,
                launch: saved.launch.clone(),
            })
            .await?;
        self.rebuild_saved_native_session(restored.session_id, &saved).await?;

        self.runtime.registry().get(restored.session_id).map(SessionRuntime::to_summary).ok_or_else(
            || BackendError::internal("restored native session is missing from registry"),
        )
    }

    pub(super) async fn save_session(
        &self,
        session_id: SessionId,
    ) -> Result<MuxCommandResult, BackendError> {
        let descriptor =
            self.runtime.registry().get(session_id).ok_or_else(|| {
                BackendError::not_found(format!("unknown session {session_id:?}"))
            })?;
        if descriptor.route.backend != BackendKind::Native {
            return Err(BackendError::unsupported(
                "save session is only implemented for native sessions in v1",
                DegradedModeReason::UnsupportedByBackend,
            ));
        }

        let session = self.runtime.attach_session(session_id).await?;
        let topology = session.topology_snapshot().await?;
        let mut screens = Vec::new();
        for pane_id in collect_pane_ids_from_topology(&topology) {
            screens.push(session.screen_snapshot(pane_id).await?);
        }

        let snapshot = SavedNativeSession {
            session_id,
            route: descriptor.route,
            title: saved_session_title(descriptor.title, &topology),
            launch: descriptor.launch,
            manifest: SavedSessionManifest::current(),
            topology,
            screens,
            saved_at_ms: SqliteSessionStore::save_timestamp_ms().map_err(|error| {
                BackendError::internal(format!("failed to prepare save timestamp - {error}"))
            })?,
        };
        self.runtime.persistence().save_native_session(&snapshot).map_err(|error| {
            BackendError::internal(format!("failed to save native session - {error}"))
        })?;

        Ok(MuxCommandResult { changed: false })
    }

    async fn rebuild_saved_native_session(
        &self,
        restored_session_id: SessionId,
        saved: &SavedNativeSession,
    ) -> Result<(), BackendError> {
        let active = ActiveSessionService::new(self.runtime);

        for saved_tab in saved.topology.tabs.iter().skip(1) {
            active
                .dispatch(
                    restored_session_id,
                    MuxCommand::NewTab(NewTabSpec { title: saved_tab.title.clone() }),
                )
                .await?;
        }

        let topology = active.topology_snapshot(restored_session_id).await?;
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
            if let Some(saved_title) = &saved_tab.title
                && live_tab.title.as_deref() != Some(saved_title.as_str())
            {
                active
                    .dispatch(
                        restored_session_id,
                        MuxCommand::RenameTab { tab_id: live_tab_id, title: saved_title.clone() },
                    )
                    .await?;
            }

            let pane_map =
                self.rebuild_saved_tab_layout(restored_session_id, live_tab_id, saved_tab).await?;
            if let Some(saved_focused_pane) = saved_tab.focused_pane
                && let Some(restored_pane_id) = pane_map.get(&saved_focused_pane).copied()
            {
                active
                    .dispatch(
                        restored_session_id,
                        MuxCommand::FocusPane { pane_id: restored_pane_id },
                    )
                    .await?;
            }

            if saved.topology.focused_tab == Some(saved_tab.tab_id) {
                restored_focus_tab_id = Some(live_tab_id);
            }
        }

        if let Some(restored_focus_tab_id) = restored_focus_tab_id {
            active
                .dispatch(
                    restored_session_id,
                    MuxCommand::FocusTab { tab_id: restored_focus_tab_id },
                )
                .await?;
        }

        Ok(())
    }

    async fn rebuild_saved_tab_layout(
        &self,
        restored_session_id: SessionId,
        live_tab_id: TabId,
        saved_tab: &TabSnapshot,
    ) -> Result<HashMap<PaneId, PaneId>, BackendError> {
        let active = ActiveSessionService::new(self.runtime);
        let topology = active.topology_snapshot(restored_session_id).await?;
        let live_tab = tab_snapshot_by_id(&topology, live_tab_id)?;
        let initial_live_pane_id = collect_pane_ids_from_node(&live_tab.root)
            .into_iter()
            .next()
            .ok_or_else(|| BackendError::internal("restored native tab has no initial pane"))?;
        let mut pane_map = HashMap::new();
        let mut pending = vec![(saved_tab.root.clone(), initial_live_pane_id)];

        while let Some((node, live_pane_id)) = pending.pop() {
            match node {
                PaneTreeNode::Leaf { pane_id } => {
                    pane_map.insert(pane_id, live_pane_id);
                }
                PaneTreeNode::Split(split) => {
                    let before = active.topology_snapshot(restored_session_id).await?;
                    let before_tab = tab_snapshot_by_id(&before, live_tab_id)?;
                    let before_panes = collect_pane_ids_from_node(&before_tab.root);
                    active
                        .dispatch(
                            restored_session_id,
                            MuxCommand::SplitPane(SplitPaneSpec {
                                pane_id: live_pane_id,
                                direction: split.direction,
                            }),
                        )
                        .await?;
                    let after = active.topology_snapshot(restored_session_id).await?;
                    let after_tab = tab_snapshot_by_id(&after, live_tab_id)?;
                    let after_panes = collect_pane_ids_from_node(&after_tab.root);
                    let new_pane_id = after_panes
                        .iter()
                        .copied()
                        .find(|pane_id| !before_panes.contains(pane_id))
                        .ok_or_else(|| {
                            BackendError::internal(
                                "restored native split did not produce a new pane id",
                            )
                        })?;

                    pending.push((*split.second, new_pane_id));
                    pending.push((*split.first, live_pane_id));
                }
            }
        }

        Ok(pane_map)
    }
}
