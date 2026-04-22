use terminal_backend_api::{
    BackendCapabilities, BackendError, BackendScope, BackendSessionSummary, CreateSessionSpec,
    DiscoveredSession,
};
use terminal_domain::{BackendKind, DegradedModeReason, RouteAuthority, SessionRoute};
use terminal_projection::SessionHealthSnapshot;

use super::runtime::SessionRuntime;

#[derive(Clone)]
pub(super) struct SessionCatalogService<'a> {
    runtime: SessionRuntime<'a>,
}

impl<'a> SessionCatalogService<'a> {
    pub(super) fn new(runtime: SessionRuntime<'a>) -> Self {
        Self { runtime }
    }

    pub(super) async fn discover_sessions(
        &self,
        backend: BackendKind,
    ) -> Result<Vec<DiscoveredSession>, BackendError> {
        self.runtime.backend(backend)?.discover_sessions(BackendScope::CurrentUser).await
    }

    pub(super) async fn backend_capabilities(
        &self,
        backend: BackendKind,
    ) -> Result<BackendCapabilities, BackendError> {
        self.runtime.backend(backend)?.capabilities().await
    }

    pub(super) async fn create_session(
        &self,
        backend: BackendKind,
        spec: CreateSessionSpec,
    ) -> Result<BackendSessionSummary, BackendError> {
        match backend {
            BackendKind::Native => self.runtime.create_native_session(spec).await,
            BackendKind::Tmux | BackendKind::Zellij => Err(BackendError::unsupported(
                "foreign backends are imported, not created",
                DegradedModeReason::ImportedForeignSession,
            )),
        }
    }

    pub(super) async fn import_session(
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
        if let Some(existing) = self.runtime.registry().get_by_route(&route) {
            return Ok(SessionRuntime::to_summary(existing));
        }

        let session_id = self.runtime.resolve_session_id_for_route(&route)?;
        self.runtime.backend(route.backend)?.attach_session(session_id, route.clone()).await?;

        let descriptor =
            crate::registry::SessionDescriptor {
                session_id,
                route,
                title,
                launch: None,
                health: SessionHealthSnapshot::ready(session_id),
            };
        let summary = SessionRuntime::to_summary(descriptor.clone());
        self.runtime.upsert_session_route(descriptor.session_id, &descriptor.route)?;
        self.runtime.registry().insert(descriptor);

        Ok(summary)
    }

    pub(super) fn available_backends(&self) -> Vec<BackendKind> {
        self.runtime.available_backends()
    }

    pub(super) fn list_sessions(&self) -> Vec<BackendSessionSummary> {
        self.runtime.list_sessions()
    }

    pub(super) fn session_count(&self) -> usize {
        self.runtime.session_count()
    }
}
