use terminal_backend_api::{
    BackendCapabilities, BackendError, BackendSessionPort, BackendSessionSummary,
    BackendSubscription, CreateSessionSpec, DiscoveredSession, MuxCommand, MuxCommandResult,
    SubscriptionSpec,
};
use terminal_domain::{
    BackendKind, DegradedModeReason, PaneId, RouteAuthority, SessionId, SessionRoute,
    imported_session_id,
};
use terminal_persistence::{SavedNativeSession, SqliteSessionStore};
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
        collect_pane_ids_from_node(&tab.root, &mut pane_ids);
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

fn collect_pane_ids_from_node(
    root: &terminal_mux_domain::PaneTreeNode,
    pane_ids: &mut Vec<PaneId>,
) {
    match root {
        terminal_mux_domain::PaneTreeNode::Leaf { pane_id } => pane_ids.push(*pane_id),
        terminal_mux_domain::PaneTreeNode::Split(split) => {
            collect_pane_ids_from_node(&split.first, pane_ids);
            collect_pane_ids_from_node(&split.second, pane_ids);
        }
    }
}
