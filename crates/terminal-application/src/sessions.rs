use std::sync::Arc;

use terminal_backend_api::{
    BackendError, BackendSessionPort, BackendSessionSummary, CreateSessionSpec, MuxBackendPort,
    MuxCommand, MuxCommandResult,
};
use terminal_domain::{BackendKind, DegradedModeReason, PaneId, SessionId};
use terminal_projection::{ScreenSnapshot, TopologySnapshot};

use crate::registry::{InMemorySessionRegistry, SessionDescriptor, SessionRegistry};

pub struct SessionService {
    native_backend: Arc<dyn MuxBackendPort>,
    registry: InMemorySessionRegistry,
}

impl SessionService {
    #[must_use]
    pub fn new(native_backend: Arc<dyn MuxBackendPort>) -> Self {
        Self { native_backend, registry: InMemorySessionRegistry::default() }
    }

    pub async fn create_session(
        &self,
        backend: BackendKind,
        spec: CreateSessionSpec,
    ) -> Result<BackendSessionSummary, BackendError> {
        match backend {
            BackendKind::Native => self.create_native_session(spec).await,
            BackendKind::Tmux | BackendKind::Zellij => Err(BackendError::unsupported(
                "foreign backends are not creatable in v1 start phase",
                DegradedModeReason::NotYetImplemented,
            )),
        }
    }

    #[must_use]
    pub fn list_sessions(&self) -> Vec<BackendSessionSummary> {
        self.registry
            .list()
            .into_iter()
            .map(|session| BackendSessionSummary {
                session_id: session.session_id,
                route: session.route,
                title: session.title,
            })
            .collect()
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

    pub async fn dispatch(
        &self,
        session_id: SessionId,
        command: MuxCommand,
    ) -> Result<MuxCommandResult, BackendError> {
        let session = self.attach_session(session_id).await?;
        session.dispatch(command).await
    }

    async fn create_native_session(
        &self,
        spec: CreateSessionSpec,
    ) -> Result<BackendSessionSummary, BackendError> {
        let binding = self.native_backend.create_session(spec.clone()).await?;
        let summary = BackendSessionSummary {
            session_id: binding.session_id,
            route: binding.route.clone(),
            title: spec.title.clone(),
        };

        self.registry.insert(SessionDescriptor {
            session_id: binding.session_id,
            route: binding.route,
            title: spec.title,
        });

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

        match descriptor.route.backend {
            BackendKind::Native => self.native_backend.attach_session(descriptor.route).await,
            BackendKind::Tmux | BackendKind::Zellij => Err(BackendError::unsupported(
                "foreign backends are not attachable in v1 start phase",
                DegradedModeReason::NotYetImplemented,
            )),
        }
    }
}
