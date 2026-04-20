use terminal_backend_api::{
    BackendCapabilities, BackendError, BackendSessionPort, BackendSessionSummary,
    BackendSubscription, CreateSessionSpec, DiscoveredSession, MuxCommand, MuxCommandResult,
    NewTabSpec, SplitPaneSpec, SubscriptionSpec,
};
use terminal_domain::{
    BackendKind, DegradedModeReason, PaneId, RouteAuthority, SavedSessionManifest, SessionId,
    SessionRoute, TabId, imported_session_id, saved_session_compatibility,
};
use terminal_mux_domain::{PaneTreeNode, TabSnapshot};
use terminal_persistence::{
    PrunedSavedSessions, SavedNativeSession, SavedSessionSummary as PersistedSavedSessionSummary,
    SqliteSessionStore,
};
use terminal_projection::{ScreenDelta, ScreenSnapshot, TopologySnapshot};

use crate::{
    backend_catalog::BackendCatalog,
    registry::{InMemorySessionRegistry, SessionDescriptor, SessionRegistry},
};

pub struct SessionService {
    backends: BackendCatalog,
    registry: InMemorySessionRegistry,
    persistence: SqliteSessionStore,
}

impl SessionService {
    #[must_use]
    pub fn new(backends: BackendCatalog) -> Self {
        let persistence =
            SqliteSessionStore::open_default().expect("default sqlite session store should open");
        Self::with_persistence(backends, persistence)
    }

    #[must_use]
    pub fn with_persistence(backends: BackendCatalog, persistence: SqliteSessionStore) -> Self {
        Self { backends, registry: InMemorySessionRegistry::default(), persistence }
    }

    pub async fn discover_sessions(
        &self,
        backend: BackendKind,
    ) -> Result<Vec<DiscoveredSession>, BackendError> {
        self.backends
            .backend(backend)?
            .discover_sessions(terminal_backend_api::BackendScope::CurrentUser)
            .await
    }

    pub async fn backend_capabilities(
        &self,
        backend: BackendKind,
    ) -> Result<BackendCapabilities, BackendError> {
        self.backends.backend(backend)?.capabilities().await
    }

    pub async fn create_session(
        &self,
        backend: BackendKind,
        spec: CreateSessionSpec,
    ) -> Result<BackendSessionSummary, BackendError> {
        match backend {
            BackendKind::Native => self.create_native_session(spec).await,
            BackendKind::Tmux | BackendKind::Zellij => Err(BackendError::unsupported(
                "foreign backends are imported, not created",
                DegradedModeReason::ImportedForeignSession,
            )),
        }
    }

    pub async fn import_session(
        &self,
        route: SessionRoute,
        title: Option<String>,
    ) -> Result<BackendSessionSummary, BackendError> {
        if route.authority != RouteAuthority::ImportedForeign {
            return Err(BackendError::invalid_input(
                "imported sessions must use imported_foreign route authority",
            ));
        }
        if route.backend == BackendKind::Native {
            return Err(BackendError::invalid_input("native sessions are created, not imported"));
        }
        if let Some(existing) = self.registry.get_by_route(&route) {
            return Ok(Self::to_summary(existing));
        }

        self.backends.backend(route.backend)?.attach_session(route.clone()).await?;

        let descriptor = SessionDescriptor {
            session_id: imported_session_id(&route)
                .ok_or_else(|| BackendError::invalid_input("route is not importable"))?,
            route,
            title,
            launch: None,
        };
        let summary = Self::to_summary(descriptor.clone());
        self.registry.insert(descriptor);

        Ok(summary)
    }

    pub fn list_saved_sessions(&self) -> Result<Vec<PersistedSavedSessionSummary>, BackendError> {
        self.persistence.list_native_sessions().map_err(|error| {
            BackendError::internal(format!("failed to list saved native sessions - {error}"))
        })
    }

    #[must_use]
    pub fn available_backends(&self) -> Vec<BackendKind> {
        self.backends.kinds()
    }

    pub fn saved_session(&self, session_id: SessionId) -> Result<SavedNativeSession, BackendError> {
        self.persistence
            .load_native_session(session_id)
            .map_err(|error| {
                BackendError::internal(format!("failed to load saved native session - {error}"))
            })?
            .ok_or_else(|| BackendError::not_found(format!("unknown saved session {session_id:?}")))
    }

    pub fn delete_saved_session(&self, session_id: SessionId) -> Result<(), BackendError> {
        let deleted = self.persistence.delete_native_session(session_id).map_err(|error| {
            BackendError::internal(format!("failed to delete saved native session - {error}"))
        })?;
        if !deleted {
            return Err(BackendError::not_found(format!("unknown saved session {session_id:?}")));
        }

        Ok(())
    }

    pub fn prune_saved_sessions(
        &self,
        keep_latest: usize,
    ) -> Result<PrunedSavedSessions, BackendError> {
        self.persistence.prune_native_sessions(keep_latest).map_err(|error| {
            BackendError::internal(format!("failed to prune saved native sessions - {error}"))
        })
    }

    pub async fn restore_saved_session(
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
            .create_native_session(CreateSessionSpec {
                title: initial_title,
                launch: saved.launch.clone(),
            })
            .await?;
        self.rebuild_saved_native_session(restored.session_id, &saved).await?;

        self.registry.get(restored.session_id).map(Self::to_summary).ok_or_else(|| {
            BackendError::internal("restored native session is missing from registry")
        })
    }

    #[must_use]
    pub fn list_sessions(&self) -> Vec<BackendSessionSummary> {
        self.registry.list().into_iter().map(Self::to_summary).collect()
    }

    #[must_use]
    pub fn session_count(&self) -> usize {
        self.registry.list().len()
    }

    pub async fn topology_snapshot(
        &self,
        session_id: SessionId,
    ) -> Result<TopologySnapshot, BackendError> {
        let session = self.attach_session(session_id).await?;
        session.topology_snapshot().await
    }

    pub async fn screen_snapshot(
        &self,
        session_id: SessionId,
        pane_id: PaneId,
    ) -> Result<ScreenSnapshot, BackendError> {
        let session = self.attach_session(session_id).await?;
        session.screen_snapshot(pane_id).await
    }

    pub async fn screen_delta(
        &self,
        session_id: SessionId,
        pane_id: PaneId,
        from_sequence: u64,
    ) -> Result<ScreenDelta, BackendError> {
        let session = self.attach_session(session_id).await?;
        session.screen_delta(pane_id, from_sequence).await
    }

    pub async fn dispatch(
        &self,
        session_id: SessionId,
        command: MuxCommand,
    ) -> Result<MuxCommandResult, BackendError> {
        if matches!(command, MuxCommand::SaveSession) {
            return self.save_session(session_id).await;
        }
        let session = self.attach_session(session_id).await?;
        let refresh_summary_title = command_updates_summary_title(&command);
        let result = session.dispatch(command).await?;
        if result.changed && refresh_summary_title {
            self.refresh_session_summary_title(session_id, &*session).await;
        }
        Ok(result)
    }

    pub async fn open_subscription(
        &self,
        session_id: SessionId,
        spec: SubscriptionSpec,
    ) -> Result<BackendSubscription, BackendError> {
        let session = self.attach_session(session_id).await?;
        session.subscribe(spec).await
    }

    async fn create_native_session(
        &self,
        spec: CreateSessionSpec,
    ) -> Result<BackendSessionSummary, BackendError> {
        let binding =
            self.backends.backend(BackendKind::Native)?.create_session(spec.clone()).await?;
        let descriptor = SessionDescriptor {
            session_id: binding.session_id,
            route: binding.route,
            title: spec.title,
            launch: spec.launch,
        };
        let summary = Self::to_summary(descriptor.clone());
        self.registry.insert(descriptor);

        Ok(summary)
    }

    async fn attach_session(
        &self,
        session_id: SessionId,
    ) -> Result<Box<dyn BackendSessionPort>, BackendError> {
        let descriptor = self
            .registry
            .get(session_id)
            .ok_or_else(|| BackendError::not_found(format!("unknown session {session_id:?}")))?;

        self.backends.backend(descriptor.route.backend)?.attach_session(descriptor.route).await
    }

    async fn save_session(&self, session_id: SessionId) -> Result<MuxCommandResult, BackendError> {
        let descriptor = self
            .registry
            .get(session_id)
            .ok_or_else(|| BackendError::not_found(format!("unknown session {session_id:?}")))?;
        if descriptor.route.backend != BackendKind::Native {
            return Err(BackendError::unsupported(
                "save session is only implemented for native sessions in v1",
                DegradedModeReason::UnsupportedByBackend,
            ));
        }

        let session = self.attach_session(session_id).await?;
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
        self.persistence.save_native_session(&snapshot).map_err(|error| {
            BackendError::internal(format!("failed to save native session - {error}"))
        })?;

        Ok(MuxCommandResult { changed: false })
    }

    async fn rebuild_saved_native_session(
        &self,
        restored_session_id: SessionId,
        saved: &SavedNativeSession,
    ) -> Result<(), BackendError> {
        for saved_tab in saved.topology.tabs.iter().skip(1) {
            self.dispatch(
                restored_session_id,
                MuxCommand::NewTab(NewTabSpec { title: saved_tab.title.clone() }),
            )
            .await?;
        }

        let topology = self.topology_snapshot(restored_session_id).await?;
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
                self.dispatch(
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
                self.dispatch(
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
            self.dispatch(
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
    ) -> Result<std::collections::HashMap<PaneId, PaneId>, BackendError> {
        let topology = self.topology_snapshot(restored_session_id).await?;
        let live_tab = tab_snapshot_by_id(&topology, live_tab_id)?;
        let initial_live_pane_id = collect_pane_ids_from_node(&live_tab.root)
            .into_iter()
            .next()
            .ok_or_else(|| BackendError::internal("restored native tab has no initial pane"))?;
        let mut pane_map = std::collections::HashMap::new();
        let mut pending = vec![(saved_tab.root.clone(), initial_live_pane_id)];

        while let Some((node, live_pane_id)) = pending.pop() {
            match node {
                PaneTreeNode::Leaf { pane_id } => {
                    pane_map.insert(pane_id, live_pane_id);
                }
                PaneTreeNode::Split(split) => {
                    let before = self.topology_snapshot(restored_session_id).await?;
                    let before_tab = tab_snapshot_by_id(&before, live_tab_id)?;
                    let before_panes = collect_pane_ids_from_node(&before_tab.root);
                    self.dispatch(
                        restored_session_id,
                        MuxCommand::SplitPane(SplitPaneSpec {
                            pane_id: live_pane_id,
                            direction: split.direction,
                        }),
                    )
                    .await?;
                    let after = self.topology_snapshot(restored_session_id).await?;
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

    async fn refresh_session_summary_title(
        &self,
        session_id: SessionId,
        session: &dyn BackendSessionPort,
    ) {
        let Some(descriptor) = self.registry.get(session_id) else {
            return;
        };
        let Ok(topology) = session.topology_snapshot().await else {
            return;
        };
        self.registry.update_title(session_id, saved_session_title(descriptor.title, &topology));
    }

    fn to_summary(session: SessionDescriptor) -> BackendSessionSummary {
        BackendSessionSummary {
            session_id: session.session_id,
            route: session.route,
            title: session.title,
        }
    }
}

fn collect_pane_ids_from_topology(topology: &TopologySnapshot) -> Vec<PaneId> {
    let mut pane_ids = Vec::new();
    for tab in &topology.tabs {
        pane_ids.extend(collect_pane_ids_from_node(&tab.root));
    }
    pane_ids
}

fn saved_session_title(
    descriptor_title: Option<String>,
    topology: &TopologySnapshot,
) -> Option<String> {
    topology
        .focused_tab
        .and_then(|focused_tab| {
            topology
                .tabs
                .iter()
                .find(|tab| tab.tab_id == focused_tab)
                .and_then(|tab| tab.title.clone())
        })
        .or_else(|| topology.tabs.iter().find_map(|tab| tab.title.clone()))
        .or(descriptor_title)
}

fn command_updates_summary_title(command: &MuxCommand) -> bool {
    matches!(
        command,
        MuxCommand::NewTab(_)
            | MuxCommand::CloseTab { .. }
            | MuxCommand::FocusTab { .. }
            | MuxCommand::RenameTab { .. }
    )
}

fn tab_snapshot_by_id(
    topology: &TopologySnapshot,
    tab_id: TabId,
) -> Result<TabSnapshot, BackendError> {
    topology
        .tabs
        .iter()
        .find(|tab| tab.tab_id == tab_id)
        .cloned()
        .ok_or_else(|| BackendError::internal(format!("missing restored tab {tab_id:?}")))
}

fn collect_pane_ids_from_node(root: &PaneTreeNode) -> Vec<PaneId> {
    let mut pane_ids = Vec::new();
    collect_pane_ids_from_node_inner(root, &mut pane_ids);
    pane_ids
}

fn collect_pane_ids_from_node_inner(root: &PaneTreeNode, pane_ids: &mut Vec<PaneId>) {
    match root {
        PaneTreeNode::Leaf { pane_id } => pane_ids.push(*pane_id),
        PaneTreeNode::Split(split) => {
            collect_pane_ids_from_node_inner(&split.first, pane_ids);
            collect_pane_ids_from_node_inner(&split.second, pane_ids);
        }
    }
}
