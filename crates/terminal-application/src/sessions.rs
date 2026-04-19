use terminal_backend_api::{
    BackendCapabilities, BackendError, BackendSessionPort, BackendSessionSummary,
    BackendSubscription, CreateSessionSpec, DiscoveredSession, MuxCommand, MuxCommandResult,
    SubscriptionSpec,
};
use terminal_domain::{
    BackendKind, DegradedModeReason, PaneId, RouteAuthority, SessionId, SessionRoute,
    imported_session_id,
};
use terminal_projection::{ScreenDelta, ScreenSnapshot, TopologySnapshot};

use crate::{
    backend_catalog::BackendCatalog,
    registry::{InMemorySessionRegistry, SessionDescriptor, SessionRegistry},
};

pub struct SessionService {
    backends: BackendCatalog,
    registry: InMemorySessionRegistry,
}

impl SessionService {
    #[must_use]
    pub fn new(backends: BackendCatalog) -> Self {
        Self { backends, registry: InMemorySessionRegistry::default() }
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
        let session = self.attach_session(session_id).await?;
        session.dispatch(command).await
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

    fn to_summary(session: SessionDescriptor) -> BackendSessionSummary {
        BackendSessionSummary {
            session_id: session.session_id,
            route: session.route,
            title: session.title,
        }
    }
}
